//! Actors' movement through their realm's physics: a capsule moved by rapier's kinematic
//! character controller each fixed tick, and drawn between the last two ticks' positions.
//!
//! The capsule is a query shape, not a collider in the world, so actors do not yet collide with
//! each other.

use bevy::prelude::*;
use panoply_core::{RealmPhysics, Viewpoint};
use panoply_scenery::SceneryPending;
use panoply_terrain::TerrainPending;
use rapier3d::{
    control::{CharacterLength, KinematicCharacterController},
    prelude::{Capsule, Pose, QueryFilter, Ray},
};

use crate::{Actor, CollisionSize};

/// Downward acceleration, in m/s².
const GRAVITY: f32 = 9.81;

/// How far up from the actor's feet the readiness ray starts, and how far down it reaches.
const GROUND_RAY_RISE: f32 = 2.0;
const GROUND_RAY_LENGTH: f32 = 1000.0;

/// Where an actor is in its realm's physics, from the fixed ticks. The actor's `Transform` is
/// drawn between `previous` and `position`, so it lags the simulation by up to a tick.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct ActorMotion {
    /// The feet, at the last tick.
    pub position: Vec3,
    /// The feet, at the tick before.
    pub previous: Vec3,
    /// Up is positive.
    pub vertical_speed: f32,
    /// Standing on something as of the last tick.
    pub grounded: bool,
}

fn controller() -> KinematicCharacterController {
    KinematicCharacterController {
        offset: CharacterLength::Absolute(0.01),
        // Surfaces whose normal's y is under 0.7 are walls (R24).
        max_slope_climb_angle: 0.7f32.acos(),
        min_slope_slide_angle: 0.7f32.acos(),
        // R25.
        snap_to_ground: Some(CharacterLength::Absolute(0.1)),
        ..default()
    }
}

/// The actor's capsule, centered on the origin.
fn capsule(size: Option<&CollisionSize>) -> Capsule {
    let size = size.cloned().unwrap_or_default();
    let radius = size.radius;
    let half_segment = (size.height * 0.5 - radius).max(0.0);
    Capsule::new_y(half_segment, radius)
}

/// Gives an actor physics once the ground under it is built. A minimal wait: nothing terrain or
/// scenery is pending anywhere, and a ray down from above the actor hits a collider, which it
/// does only after the realm has stepped since the collider was inserted. TODO(X33): the
/// footprint query.
pub(crate) fn start_actor_physics(
    mut commands: Commands,
    actors: Query<(Entity, &Actor, &Transform), Without<ActorMotion>>,
    realms: Query<&RealmPhysics>,
    terrain: TerrainPending,
    scenery: SceneryPending,
) {
    if actors.is_empty() || terrain.waiting_on().is_some() || scenery.waiting_on().is_some() {
        return;
    }
    for (entity, actor, transform) in &actors {
        let Ok(physics) = realms.get(actor.realm) else {
            continue;
        };
        let position = transform.translation;
        let ray = Ray::new(position + Vec3::Y * GROUND_RAY_RISE, Vec3::NEG_Y);
        let ground = physics
            .world()
            .cast_ray(&ray, GROUND_RAY_LENGTH, true, QueryFilter::default());
        if ground.is_some() {
            commands.entity(entity).insert(ActorMotion {
                position,
                previous: position,
                vertical_speed: 0.0,
                grounded: false,
            });
        }
    }
}

/// Moves actors one fixed tick: falling under gravity, stopped and slid by what they meet, and
/// pushed out of what they are inside (R26). Runs after the realm steps, so colliders inserted
/// since the last tick are in its broad phase. Only the viewpoint's realm steps, so actors in
/// other realms hold still.
pub(crate) fn move_actors(
    mut actors: Query<(&Actor, &mut ActorMotion, Option<&CollisionSize>)>,
    realms: Query<&RealmPhysics>,
    viewpoint: Res<Viewpoint>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.timestep().as_secs_f32();
    let controller = controller();
    for (actor, mut motion, size) in &mut actors {
        motion.previous = motion.position;
        if viewpoint.realm != Some(actor.realm) {
            continue;
        }
        let Ok(physics) = realms.get(actor.realm) else {
            continue;
        };

        let shape = capsule(size);
        let center = motion.position + Vec3::Y * (shape.half_height() + shape.radius);
        // A grounded actor asks for no movement, which is when the controller depenetrates.
        if !motion.grounded {
            motion.vertical_speed -= GRAVITY * dt;
        }
        let movement = controller.move_shape(
            dt,
            &physics.world().query_pipeline(),
            &shape,
            &Pose::from_translation(center),
            Vec3::Y * motion.vertical_speed * dt,
            |_| {},
        );
        motion.position += movement.translation;
        motion.grounded = movement.grounded;
        if motion.grounded {
            motion.vertical_speed = 0.0;
        }
    }
}

/// Draws each actor between its last two ticks' positions.
pub(crate) fn interpolate_actors(
    mut actors: Query<(&ActorMotion, &mut Transform)>,
    time: Res<Time<Fixed>>,
) {
    let fraction = time.overstep_fraction();
    for (motion, mut transform) in &mut actors {
        transform.translation = motion.previous.lerp(motion.position, fraction);
    }
}
