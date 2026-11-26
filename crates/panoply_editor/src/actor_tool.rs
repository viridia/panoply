//! Scenery mode's actor tool: a press places the chosen actor where the pointer is, its capsule
//! landed on what is under it, and a drag turns it toward the pointer before the release puts it in
//! the precinct ([`placement`](crate::placement)). Each placed actor is given an id, named for its
//! exemplar. Hovering shows the actor where a click would place it, idling, with its footprint and
//! an arrow along its facing. The tool only places: picking what is there is selection mode's.

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use bevy_action_map::prelude::*;
use panoply_actors::{
    capsule, Actor, ActorHeld, ActorMotion, ActorRebuildAspects, ActorScale, CollisionSize,
};
use panoply_core::{Realm, ToRapier, Viewpoint};
use panoply_exemplar::{Exemplar, InstanceAspects};
use panoply_overlays::{
    AddOverlay, OverlayColor, OverlayShape, PolygonOptions, ShapeBuilder, StrokeMarker,
};
use panoply_precinct::PrecinctAsset;
use panoply_scenery::precinct::Precinct;
use rapier3d::prelude::{Pose, SharedShape};

use crate::{
    exemplar_list::{chosen_placeable, SceneryChoices},
    mode_selector::SceneryTool,
    placement::{turn_facing, DialPlace, PlacementDisplaySystems, PlacementGesture},
    precinct_edit::PrecinctEdits,
    scenery_pointer::{RotateLeft, RotateRight, SceneryKeys, SceneryPointerSystems},
    shell::SAVE_DELAY,
    toast::ShowToast,
};

/// The actor tool's options, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, Copy, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "actor_tool")]
pub struct ActorToolSettings {
    /// The facing a click places actors with, in degrees as a precinct holds it. The keys turn
    /// it; a drag turns only the actor it places.
    pub facing: f32,
}

/// The actor a release would place, and where, which the preview shows.
#[derive(Resource, Clone, Copy, PartialEq, Default)]
struct ActorTarget {
    /// `None` while nothing placeable is chosen.
    exemplar: Option<AssetId<Exemplar>>,
    /// In the world's coordinates, and the facing in degrees; `None` while the pointer is over
    /// nothing.
    place: Option<(Vec3, f32)>,
}

/// The preview: an actor of the chosen exemplar, held where the tool puts it (`ActorHeld`), not
/// in any precinct.
#[derive(Component)]
struct ActorPreview;

/// How far the footprint's arrow reaches, as a multiple of the capsule's radius.
const ARROW_LENGTH: f32 = 1.6;

/// The chosen actor where a release would place it: its capsule's outline on the ground, and,
/// while no drag shows the dial, an arrow along its facing, in degrees as a precinct holds it.
/// Hidden while `None`.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct ActorFootprint(pub Option<(DialPlace, f32, bool)>);

impl OverlayShape for ActorFootprint {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let Some((place, facing, arrow)) = self.0 else {
            return;
        };
        let builder = place.builder(builder);
        builder.stroke_circle(place.center.xz(), place.radius, 32);
        if arrow {
            builder.stroke_polygon(
                &[
                    place.center.xz(),
                    place.toward(facing, place.radius * ARROW_LENGTH),
                ],
                PolygonOptions {
                    end_marker: StrokeMarker::Arrowhead,
                    ..default()
                },
            );
        }
    }
}

pub struct ActorToolPlugin;

impl Plugin for ActorToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorTarget>()
            .register_type::<ActorToolSettings>()
            .add_overlay::<ActorFootprint>()
            .add_systems(OnEnter(SceneryTool::PlaceActor), spawn_footprint)
            .add_systems(
                Update,
                (rotate_keys, update_actor_tool, show_actor_preview)
                    .chain()
                    .after(SceneryPointerSystems)
                    .before(PlacementDisplaySystems)
                    .run_if(in_state(SceneryTool::PlaceActor)),
            );
    }
}

fn spawn_footprint(mut commands: Commands) {
    commands.spawn((
        Name::new("ActorFootprint"),
        ActorFootprint::default(),
        OverlayColor {
            base: css::LIGHT_STEEL_BLUE.with_alpha(0.9).into(),
            underlay: 0.6,
        },
        Propagate(RenderLayers::none()),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(SceneryTool::PlaceActor),
    ));
}

/// Turns the actors to place a quarter turn, as the fixture tool's keys turn fixtures.
fn rotate_keys(
    input: ContextActions<SceneryKeys>,
    mut settings: ResMut<ActorToolSettings>,
    mut commands: Commands,
) {
    let turns = input.fired::<RotateRight>() as i8 - input.fired::<RotateLeft>() as i8;
    if turns != 0 {
        settings.facing = turn_facing(settings.facing, turns);
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// Follows the pointer and the chosen actor, editing the precinct as a drag ends, and shows the
/// actor's footprint.
fn update_actor_tool(
    mut gesture: PlacementGesture,
    settings: Res<ActorToolSettings>,
    choices: Res<SceneryChoices>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    precincts: Query<&Precinct>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    mut edits: PrecinctEdits,
    mut footprints: Query<(&mut ActorFootprint, &mut Propagate<RenderLayers>)>,
    mut target: ResMut<ActorTarget>,
    mut commands: Commands,
) {
    let actor = chosen_placeable(&choices, SceneryTool::PlaceActor, &server, &exemplars);
    let exemplar = actor.clone().ok().flatten();
    // The capsule the actor gets, its base at the actor's feet.
    let shape = exemplar.map(|id| {
        capsule(
            Exemplar::find_aspect::<CollisionSize>(&exemplars, id),
            Exemplar::find_aspect::<ActorScale>(&exemplars, id),
        )
    });
    let shapes: Vec<_> = shape
        .iter()
        .map(|shape| {
            let center = Vec3::Y * (shape.half_height() + shape.radius);
            (
                SharedShape::new(*shape),
                Pose::from_translation(center.to_rapier()),
            )
        })
        .collect();
    let frame = gesture.update(&shapes, settings.facing);

    if let Some(done) = frame.released {
        match &actor {
            Ok(Some(id)) => {
                let path = server.get_path(*id).map(|path| path.to_string());
                if let (Ok(precinct), Some(path)) = (precincts.get(done.precinct), path) {
                    let name = choices.choice(SceneryTool::PlaceActor).unwrap_or_default();
                    edits.edit_precinct(done.precinct, precinct, |asset: &mut PrecinctAsset| {
                        asset.add_actor(path, done.facing, done.dial.center);
                        Some(format!("Place {name}").into())
                    });
                }
            }
            Err(Some(reason)) => {
                commands.write_message(ShowToast(reason.clone().into()));
            }
            Ok(None) | Err(None) => {}
        }
    }

    let layer = viewpoint
        .realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    let place = frame.target.zip(shape).map(|((center, facing), shape)| {
        let place = DialPlace {
            center,
            radius: shape.radius,
        };
        (place, facing, !gesture.dragging())
    });
    for (mut footprint, mut propagate) in &mut footprints {
        footprint.set_if_neq(ActorFootprint(place));
        propagate.set_if_neq(Propagate(layer.clone()));
    }
    target.set_if_neq(ActorTarget {
        exemplar,
        place: frame.target,
    });
}

/// Shows the chosen actor where a release would place it, replacing the preview when the choice or
/// the viewed realm changes.
fn show_actor_preview(
    target: Res<ActorTarget>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    server: Res<AssetServer>,
    mut previews: Query<(Entity, &Actor, &mut ActorMotion, &mut Visibility), With<ActorPreview>>,
    mut commands: Commands,
) {
    let wanted = target.exemplar.zip(viewpoint.realm);
    let shown = if target.place.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let (position, facing) = target.place.unwrap_or_default();
    let facing = facing.to_radians();
    if let Ok((entity, actor, mut motion, mut visibility)) = previews.single_mut() {
        if Some((actor.exemplar.id(), actor.realm)) == wanted {
            visibility.set_if_neq(shown);
            if motion.position != position || motion.facing != facing {
                // Both ticks alike, so it is drawn there, standing still: no gait or turn plays.
                motion.position = position;
                motion.previous = position;
                motion.facing = facing;
                motion.previous_facing = facing;
                motion.last_stood = position;
            }
            return;
        }
        commands.entity(entity).despawn();
    }
    let Some((id, realm)) = wanted else {
        return;
    };
    let Some(exemplar) = server.get_id_handle(id) else {
        return;
    };
    let layer = realms
        .get(realm)
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    commands.spawn((
        Name::new("ActorPreview"),
        ActorPreview,
        Actor { exemplar, realm },
        ActorHeld,
        ActorMotion {
            position,
            previous: position,
            facing,
            previous_facing: facing,
            vertical_speed: 0.,
            grounded: true,
            ground_normal: Vec3::Y,
            falling: false,
            last_stood: position,
            push: Vec2::ZERO,
            stalled_ticks: 0,
            stalled_from: position,
        },
        InstanceAspects::default(),
        ActorRebuildAspects,
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(facing)),
        shown,
        Propagate(layer),
        Propagate(Pickable::IGNORE),
        DespawnOnExit(SceneryTool::PlaceActor),
    ));
}
