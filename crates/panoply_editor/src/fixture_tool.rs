//! Scenery mode's fixture tool: a press places the chosen fixture where the pointer is, landed on
//! what is under it ([`placement`](crate::placement)), and a drag turns it toward the pointer, on
//! a dial, before the release puts it in the precinct. Hovering shows the fixture where a click
//! would place it. The tool only places: picking what is there is selection mode's. Escape during a
//! drag cancels it.

use std::borrow::Cow;

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    ecs::system::SystemParam,
    picking::{
        events::PointerState,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
    window::PrimaryWindow,
};
use bevy_action_map::prelude::*;
use panoply_core::{Realm, RealmPhysics, Viewpoint};
use panoply_exemplar::Exemplar;
use panoply_overlays::{AddOverlay, OverlayColor};
use panoply_precinct::{PrecinctAsset, SceneryInstanceId};
use panoply_scenery::{
    precinct::Precinct,
    scenery_element::{SceneryElement, SceneryElementRebuildAspects},
    ColliderDesc, PrecinctCache, SceneryColliders, PRECINCT_SIZE_F,
};

use crate::{
    exemplar_list::{chosen_placeable, SceneryChoices},
    mode_selector::SceneryTool,
    placement::{
        cast_placement, dial_radius, facing_toward, is_quarter_turn, DialPlace,
        FacingDial, FacingHand,
    },
    precinct_edit::PrecinctEdits,
    scenery_pointer::{
        precinct_at, RotateLeft, RotateRight, SceneryKeys, SceneryPointer, SceneryPointerSystems,
    },
    shell::SAVE_DELAY,
    terrain_cursor::level_point,
    toast::ShowToast,
};

/// The fixture tool's options, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, Copy, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "fixture_tool")]
pub struct FixtureToolSettings {
    /// The facing a click places fixtures with, in degrees as a precinct holds it. The keys turn
    /// it; a drag turns only the fixture it places.
    pub facing: f32,
}

impl FixtureToolSettings {
    /// Turns by `turns` quarter turns, right for positive, as the wall tool turns. A facing between
    /// quarter turns takes the next one that way first, so the keys always return to them.
    fn turn(&mut self, turns: i8) {
        let quarters = self.facing / 90.;
        let next = if turns > 0 {
            quarters.ceil() - turns as f32
        } else {
            quarters.floor() - turns as f32
        };
        self.facing = (next * 90.).rem_euclid(360.) + 0.;
    }
}

/// A drag in progress, held from the press to the release.
#[derive(Clone, Copy, Debug)]
struct FixtureDrag {
    /// The precinct the fixture goes in, and its coordinates.
    precinct: Entity,
    coords: IVec2,
    /// Where the fixture goes, in the world's coordinates, and its dial's radius.
    dial: DialPlace,
    /// In degrees, as a precinct holds it.
    facing: f32,
    /// Where the press was in the window, in logical pixels, and whether the pointer has moved
    /// far enough from it to turn the fixture. Until it has, the fixture keeps the tool's facing:
    /// the pointer's point on the fixture's level can be far from the fixture at the press, as on
    /// a click on a wall, whose fixture lands at the wall's foot.
    press: Option<Vec2>,
    turning: bool,
    /// Escape cancelled it: the release places nothing.
    cancelled: bool,
}

/// How far the pointer must move from the press, in logical pixels, before a drag turns the
/// fixture.
const TURN_THRESHOLD: f32 = 5.;

/// The drag in progress, if any; read by `FixtureDragKeys`.
#[derive(Resource, Default)]
struct FixtureDragState(Option<FixtureDrag>);

/// Escape cancels a drag. Above `AppControl`, whose Escape quits, and consumed, so cancelling a
/// drag never quits.
#[derive(InputContext)]
#[context(path = "editor.fixture_drag", tick = Render, priority = 500)]
pub struct FixtureDragKeys;

/// Cancel the drag: its release places nothing.
#[derive(InputAction)]
#[action(path = "editor.fixture_drag.cancel", output = bool, intent = Button)]
pub struct CancelFixtureDrag;

/// The fixture a release would place, and where, which the preview shows.
#[derive(Resource, Clone, Copy, PartialEq, Default)]
struct FixtureTarget {
    /// `None` while nothing placeable is chosen.
    exemplar: Option<AssetId<Exemplar>>,
    /// In the world's coordinates; `None` while the pointer is over nothing.
    position: Option<Vec3>,
    /// In degrees.
    facing: f32,
    /// The dial, from the press, so it shows that a drag would turn the fixture.
    dial: Option<DialPlace>,
}

/// The root of the preview, at the world's origin. Not a precinct's child, so its rebuild leaves
/// the preview alone, and the preview gets no colliders.
#[derive(Component)]
struct FixturePreview;

/// The preview's element, whose colliders, from its exemplar's aspects, are what the tool casts.
#[derive(Component)]
struct FixturePreviewElement;

/// The facing, as text beside the pointer, while a drag turns the fixture.
#[derive(Component)]
struct FacingReadout;

/// The readout's offset from the pointer, in pixels, and its font size.
const READOUT_OFFSET: Vec2 = Vec2::new(18., 14.);
const READOUT_FONT_SIZE: f32 = 14.;

pub struct FixtureToolPlugin;

impl Plugin for FixtureToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FixtureDragState>()
            .init_resource::<FixtureTarget>()
            .register_type::<FixtureToolSettings>()
            .add_overlay::<FacingDial>()
            .add_overlay::<FacingHand>()
            .add_context::<FixtureDragKeys>(|context| {
                context.active_if(|drag: Res<FixtureDragState>| drag.0.is_some());
                context.bind::<CancelFixtureDrag>(KeyCode::Escape).consume();
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("FixtureDragKeys"), FixtureDragKeys));
            })
            .add_systems(OnEnter(SceneryTool::PlaceFixture), spawn_fixture_tool)
            .add_systems(OnExit(SceneryTool::PlaceFixture), clear_drag)
            .add_systems(
                Update,
                (
                    rotate_keys,
                    update_fixture_tool,
                    show_fixture_target,
                    show_readout,
                )
                    .chain()
                    .after(SceneryPointerSystems)
                    .run_if(in_state(SceneryTool::PlaceFixture)),
            );
    }
}

fn spawn_fixture_tool(asset_server: Res<AssetServer>, mut commands: Commands) {
    let overlay = |color: Srgba| {
        (
            OverlayColor {
                base: color.with_alpha(0.9).into(),
                underlay: 0.6,
            },
            Propagate(RenderLayers::none()),
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(SceneryTool::PlaceFixture),
        )
    };
    commands.spawn((
        Name::new("FacingDial"),
        FacingDial::default(),
        overlay(css::SILVER),
    ));
    commands.spawn((
        Name::new("FacingHand"),
        FacingHand::default(),
        overlay(css::SILVER),
    ));
    commands.spawn((
        Name::new("FixturePreview"),
        FixturePreview,
        Transform::default(),
        Visibility::Hidden,
        Propagate(Pickable::IGNORE),
        DespawnOnExit(SceneryTool::PlaceFixture),
    ));
    commands.spawn((
        Name::new("FacingReadout"),
        FacingReadout,
        Text::default(),
        TextFont {
            font: FontSource::Handle(asset_server.load("fonts/Rubik/Rubik-VariableFont_wght.ttf")),
            font_size: FontSize::Px(READOUT_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Pickable::IGNORE,
        DespawnOnExit(SceneryTool::PlaceFixture),
    ));
}

fn clear_drag(mut drag: ResMut<FixtureDragState>) {
    drag.0 = None;
}

/// Turns the fixtures to place a quarter turn, as the wall tool's keys turn walls.
fn rotate_keys(
    input: ContextActions<SceneryKeys>,
    mut settings: ResMut<FixtureToolSettings>,
    mut commands: Commands,
) {
    let turns = input.fired::<RotateRight>() as i8 - input.fired::<RotateLeft>() as i8;
    if turns != 0 {
        settings.turn(turns);
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// What the tool reads to land a fixture: the viewed realm's physics, its precincts, and the
/// preview's colliders.
#[derive(SystemParam)]
struct Landing<'w, 's> {
    viewpoint: Res<'w, Viewpoint>,
    realms: Query<'w, 's, &'static RealmPhysics>,
    cache: Res<'w, PrecinctCache>,
    previews: Query<
        'w,
        's,
        (&'static SceneryElement, Option<&'static SceneryColliders>),
        With<FixturePreviewElement>,
    >,
    precincts: Query<'w, 's, &'static Precinct>,
}

/// Follows the pointer and the chosen fixture; starts, turns, cancels and ends drags, editing the
/// precinct as one ends.
fn update_fixture_tool(
    pointer: Res<SceneryPointer>,
    settings: Res<FixtureToolSettings>,
    choices: Res<SceneryChoices>,
    pointers: Res<PointerState>,
    keys: ContextActions<FixtureDragKeys>,
    mut drag: ResMut<FixtureDragState>,
    mut was_pressed: Local<bool>,
    ui_nodes: Query<(), With<Node>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    landing: Landing,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut edits: PrecinctEdits,
    mut target: ResMut<FixtureTarget>,
    mut commands: Commands,
) {
    // A press in the view, not on the panel or a widget over the view.
    let pressed = pointers
        .get(PointerId::Mouse, PointerButton::Primary)
        .is_some_and(|state| state.pressing.keys().any(|e| !ui_nodes.contains(*e)));
    let just_pressed = pressed && !*was_pressed;
    let released = !pressed && *was_pressed;
    *was_pressed = pressed;

    if keys.fired::<CancelFixtureDrag>() {
        if let Some(drag) = drag.0.as_mut() {
            drag.cancelled = true;
        }
    }

    let fixture = chosen_placeable(&choices, SceneryTool::PlaceFixture, &server, &exemplars);
    let exemplar = fixture.clone().ok().flatten();
    // The chosen fixture's shapes, once the preview shows it.
    let shapes: Vec<_> = landing
        .previews
        .iter()
        .filter(|(element, _)| Some(element.exemplar.id()) == exemplar)
        .flat_map(|(_, colliders)| colliders.into_iter().flat_map(|c| c.0.iter()))
        .filter_map(ColliderDesc::shape)
        .collect();
    let realm = landing.viewpoint.realm;
    let world = realm
        .and_then(|realm| landing.realms.get(realm).ok())
        .map(RealmPhysics::world);
    let land = |facing: f32| {
        world
            .zip(pointer.ray)
            .and_then(|(world, ray)| cast_placement(world, &shapes, facing.to_radians(), ray))
            .or(pointer.point)
    };

    let cursor = windows.single().ok().and_then(Window::cursor_position);
    if just_pressed {
        drag.0 = land(settings.facing).and_then(|position| {
            let (precinct, coords) = precinct_at(&landing.cache, realm?, position)?;
            Some(FixtureDrag {
                precinct,
                coords,
                dial: DialPlace {
                    center: position,
                    radius: dial_radius(&shapes),
                },
                facing: settings.facing,
                press: cursor,
                turning: false,
                cancelled: false,
            })
        });
    } else if let Some(drag) = drag.0.as_mut() {
        drag.turning |= cursor
            .zip(drag.press)
            .is_some_and(|(cursor, press)| cursor.distance(press) >= TURN_THRESHOLD);
        let dial = drag.dial;
        let point = pointer
            .ray
            .filter(|_| drag.turning)
            .and_then(|ray| level_point(ray, dial.center.y));
        if let Some(facing) =
            point.and_then(|point| facing_toward(dial.center.xz(), point.xz(), dial.radius))
        {
            drag.facing = facing;
        }
    }

    if released {
        if let Some(done) = drag.0.take().filter(|drag| !drag.cancelled) {
            match &fixture {
                Ok(Some(id)) => {
                    let path = server.get_path(*id).map(|path| path.to_string());
                    if let (Ok(precinct), Some(path)) =
                        (landing.precincts.get(done.precinct), path)
                    {
                        let name = choices.choice(SceneryTool::PlaceFixture).unwrap_or_default();
                        let origin = (done.coords.as_vec2() * PRECINCT_SIZE_F).extend(0.).xzy();
                        let position = done.dial.center - origin;
                        edits.edit(&precinct.asset, |asset| {
                            place_fixture(asset, path, &name, done.facing, position)
                        });
                    }
                }
                Err(Some(reason)) => {
                    commands.write_message(ShowToast(reason.clone().into()));
                }
                Ok(None) | Err(None) => {}
            }
        }
    }

    target.set_if_neq(match drag.0 {
        Some(drag) if !drag.cancelled => FixtureTarget {
            exemplar,
            position: Some(drag.dial.center),
            facing: drag.facing,
            dial: Some(drag.dial),
        },
        Some(_) => FixtureTarget::default(),
        None => FixtureTarget {
            exemplar,
            position: land(settings.facing),
            facing: settings.facing,
            dial: None,
        },
    });
}

/// Shows the [`FixtureTarget`]: the chosen fixture where it would go, and the dial while a drag
/// turns it.
fn show_fixture_target(
    target: Res<FixtureTarget>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    server: Res<AssetServer>,
    mut dials: Query<(&mut FacingDial, &mut Propagate<RenderLayers>), Without<FacingHand>>,
    mut hands: Query<(
        &mut FacingHand,
        &mut OverlayColor,
        &mut Propagate<RenderLayers>,
    )>,
    mut previews: Query<(Entity, &mut Visibility, Option<&Children>), With<FixturePreview>>,
    mut elements: Query<(&mut SceneryElement, &mut Transform), With<FixturePreviewElement>>,
    mut commands: Commands,
) {
    let layer = viewpoint
        .realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    for (mut dial, mut propagate) in &mut dials {
        dial.set_if_neq(FacingDial(target.dial));
        propagate.set_if_neq(Propagate(layer.clone()));
    }
    let snapped = is_quarter_turn(target.facing);
    for (mut hand, mut color, mut propagate) in &mut hands {
        hand.set_if_neq(FacingHand(target.dial.map(|dial| (dial, target.facing))));
        let base = if snapped { css::GOLD } else { css::SILVER };
        color.base = base.with_alpha(0.9).into();
        propagate.set_if_neq(Propagate(layer.clone()));
    }

    let Ok((root, mut visibility, children)) = previews.single_mut() else {
        return;
    };
    let shown = target.exemplar.is_some() && target.position.is_some();
    visibility.set_if_neq(if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    let position = target.position.unwrap_or_default();
    let facing = target.facing.to_radians();
    let transform = Transform::from_translation(position).with_rotation(Quat::from_rotation_y(facing));
    let child = children.and_then(|children| children.first().copied());
    let current = child.and_then(|child| elements.get_mut(child).ok());
    match (current, target.exemplar) {
        (Some((mut element, mut element_transform)), Some(id)) if element.exemplar.id() == id => {
            if element.position != position || element.facing != facing {
                element.position = position;
                element.facing = facing;
            }
            element_transform.set_if_neq(transform);
        }
        (_, exemplar) => {
            if let Some(child) = child {
                commands.entity(child).despawn();
            }
            let Some(handle) = exemplar.and_then(|id| server.get_id_handle(id)) else {
                return;
            };
            commands.spawn((
                Name::new("FixturePreviewElement"),
                FixturePreviewElement,
                SceneryElement {
                    iid: SceneryInstanceId::None,
                    exemplar: handle,
                    facing,
                    position,
                },
                transform,
                Visibility::Inherited,
                layer,
                SceneryElementRebuildAspects,
                ChildOf(root),
            ));
        }
    }
}

/// Shows the facing beside the pointer while a drag turns the fixture.
fn show_readout(
    target: Res<FixtureTarget>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut readouts: Query<(&mut Text, &mut Node), With<FacingReadout>>,
) {
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    for (mut text, mut node) in &mut readouts {
        let Some(cursor) = cursor.filter(|_| target.dial.is_some()) else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        let place = cursor + READOUT_OFFSET;
        if node.display != Display::Flex || node.left != px(place.x) || node.top != px(place.y) {
            node.display = Display::Flex;
            node.left = px(place.x);
            node.top = px(place.y);
        }
        let shown = format!("{}°", target.facing.round());
        if text.0 != shown {
            text.0 = shown;
        }
    }
}

/// Places a fixture of the exemplar at `path`, named `name`, at `position` in the precinct's
/// coordinates, facing `facing` degrees. Returns the edit's name.
fn place_fixture(
    precinct: &mut PrecinctAsset,
    path: String,
    name: &str,
    facing: f32,
    position: Vec3,
) -> Option<Cow<'static, str>> {
    let index = precinct
        .scenery_type_index(&path)
        .unwrap_or_else(|| precinct.add_scenery_type(path));
    precinct.add_scenery_element(index, facing, position, None);
    Some(format!("Place {name}").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placing_adds_one_fixture_of_its_type() {
        let mut precinct = PrecinctAsset {
            scenery_types: vec!["fixtures#Chair".into()],
            ..default()
        };
        let at = Vec3::new(3.2, 0.52, 7.9);
        let label = place_fixture(&mut precinct, "fixtures#Bed".into(), "Bed", 35., at);
        assert_eq!(label.as_deref(), Some("Place Bed"));
        place_fixture(&mut precinct, "fixtures#Chair".into(), "Chair", 90., at);
        assert_eq!(precinct.scenery_types.len(), 2);
        let placed: Vec<(usize, f32, Vec3)> = precinct
            .scenery
            .iter()
            .map(|s| (s.id, s.facing, s.position))
            .collect();
        assert_eq!(placed, [(1, 35., at), (0, 90., at)]);
    }

    #[test]
    fn keys_turn_by_quarter_turns_as_the_wall_tool_turns() {
        let mut settings = FixtureToolSettings { facing: 0. };
        settings.turn(1);
        assert_eq!(settings.facing, 270.);
        settings.turn(-2);
        assert_eq!(settings.facing, 90.);
    }

    #[test]
    fn keys_return_a_facing_between_quarter_turns_to_one() {
        let turned = |facing, turns| {
            let mut settings = FixtureToolSettings { facing };
            settings.turn(turns);
            settings.facing
        };
        assert_eq!(turned(35., 1), 0.);
        assert_eq!(turned(35., -1), 90.);
        assert_eq!(turned(305., -1), 0.);
        assert_eq!(turned(35., 2), 270.);
    }
}
