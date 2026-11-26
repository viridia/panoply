//! Scenery mode's fixture tool: a press places the chosen fixture where the pointer is, landed on
//! what is under it, and a drag turns it toward the pointer before the release puts it in the
//! precinct ([`placement`](crate::placement)). Hovering shows the fixture where a click would
//! place it. The tool only places: picking what is there is selection mode's.

use std::borrow::Cow;

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use bevy_action_map::prelude::*;
use panoply_core::{Realm, Viewpoint};
use panoply_exemplar::Exemplar;
use panoply_precinct::PrecinctAsset;
use panoply_scenery::{
    precinct::Precinct,
    scenery_element::{SceneryElement, SceneryElementRebuildAspects},
    ColliderDesc, SceneryColliders, PRECINCT_SIZE_F,
};

use crate::{
    exemplar_list::{chosen_placeable, SceneryChoices},
    mode_selector::SceneryTool,
    placement::{turn_facing, PlacementDisplaySystems, PlacementGesture},
    precinct_edit::PrecinctEdits,
    scenery_pointer::{RotateLeft, RotateRight, SceneryKeys, SceneryPointerSystems},
    shell::SAVE_DELAY,
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

/// The fixture a release would place, and where, which the preview shows.
#[derive(Resource, Clone, Copy, PartialEq, Default)]
struct FixtureTarget {
    /// `None` while nothing placeable is chosen.
    exemplar: Option<AssetId<Exemplar>>,
    /// In the world's coordinates, and the facing in degrees; `None` while the pointer is over
    /// nothing.
    place: Option<(Vec3, f32)>,
}

/// The root of the preview, at the world's origin. Not a precinct's child, so its rebuild leaves
/// the preview alone, and the preview gets no colliders.
#[derive(Component)]
struct FixturePreview;

/// The preview's element, whose colliders, from its exemplar's aspects, are what the tool casts.
#[derive(Component)]
struct FixturePreviewElement;

pub struct FixtureToolPlugin;

impl Plugin for FixtureToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FixtureTarget>()
            .register_type::<FixtureToolSettings>()
            .add_systems(OnEnter(SceneryTool::PlaceFixture), spawn_fixture_preview)
            .add_systems(
                Update,
                (rotate_keys, update_fixture_tool, show_fixture_target)
                    .chain()
                    .after(SceneryPointerSystems)
                    .before(PlacementDisplaySystems)
                    .run_if(in_state(SceneryTool::PlaceFixture)),
            );
    }
}

fn spawn_fixture_preview(mut commands: Commands) {
    commands.spawn((
        Name::new("FixturePreview"),
        FixturePreview,
        Transform::default(),
        Visibility::Hidden,
        Propagate(Pickable::IGNORE),
        DespawnOnExit(SceneryTool::PlaceFixture),
    ));
}

/// Turns the fixtures to place a quarter turn, as the wall tool's keys turn walls.
fn rotate_keys(
    input: ContextActions<SceneryKeys>,
    mut settings: ResMut<FixtureToolSettings>,
    mut commands: Commands,
) {
    let turns = input.fired::<RotateRight>() as i8 - input.fired::<RotateLeft>() as i8;
    if turns != 0 {
        settings.facing = turn_facing(settings.facing, turns);
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// Follows the pointer and the chosen fixture, editing the precinct as a drag ends.
fn update_fixture_tool(
    mut gesture: PlacementGesture,
    settings: Res<FixtureToolSettings>,
    choices: Res<SceneryChoices>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    previews: Query<(&SceneryElement, Option<&SceneryColliders>), With<FixturePreviewElement>>,
    precincts: Query<&Precinct>,
    mut edits: PrecinctEdits,
    mut target: ResMut<FixtureTarget>,
    mut commands: Commands,
) {
    let fixture = chosen_placeable(&choices, SceneryTool::PlaceFixture, &server, &exemplars);
    let exemplar = fixture.clone().ok().flatten();
    // The chosen fixture's shapes, once the preview shows it.
    let shapes: Vec<_> = previews
        .iter()
        .filter(|(element, _)| Some(element.exemplar.id()) == exemplar)
        .flat_map(|(_, colliders)| colliders.into_iter().flat_map(|c| c.0.iter()))
        .filter_map(ColliderDesc::shape)
        .collect();
    let frame = gesture.update(&shapes, settings.facing);

    if let Some(done) = frame.released {
        match &fixture {
            Ok(Some(id)) => {
                let path = server.get_path(*id).map(|path| path.to_string());
                if let (Ok(precinct), Some(path)) = (precincts.get(done.precinct), path) {
                    let name = choices
                        .choice(SceneryTool::PlaceFixture)
                        .unwrap_or_default();
                    let origin = (done.coords.as_vec2() * PRECINCT_SIZE_F).extend(0.).xzy();
                    let position = done.dial.center - origin;
                    edits.edit_precinct(done.precinct, precinct, |asset| {
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

    target.set_if_neq(FixtureTarget {
        exemplar,
        place: frame.target,
    });
}

/// Shows the chosen fixture where a release would place it.
fn show_fixture_target(
    target: Res<FixtureTarget>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    server: Res<AssetServer>,
    mut previews: Query<(Entity, &mut Visibility, Option<&Children>), With<FixturePreview>>,
    mut elements: Query<(&mut SceneryElement, &mut Transform), With<FixturePreviewElement>>,
    mut commands: Commands,
) {
    let Ok((root, mut visibility, children)) = previews.single_mut() else {
        return;
    };
    let shown = target.exemplar.is_some() && target.place.is_some();
    visibility.set_if_neq(if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    let (position, facing) = target.place.unwrap_or_default();
    let facing = facing.to_radians();
    let transform =
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(facing));
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
            let layer = viewpoint
                .realm
                .and_then(|e| realms.get(e).ok())
                .map_or(RenderLayers::none(), |r| r.layer.clone());
            commands.spawn((
                Name::new("FixturePreviewElement"),
                FixturePreviewElement,
                SceneryElement {
                    exemplar: handle,
                    facing,
                    position,
                    ..default()
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
}
