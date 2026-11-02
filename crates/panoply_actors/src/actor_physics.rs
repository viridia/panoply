//! Actors' movement through their realm's physics: a capsule moved by rapier's kinematic
//! character controller each fixed tick, and drawn between the last two ticks' poses.
//!
//! The capsule is a query shape, not a collider in the world, so actors do not yet collide with
//! each other.

use bevy::prelude::*;
use panoply_core::{RealmPhysics, Viewpoint};
use panoply_scenery::SceneryPending;
use panoply_terrain::TerrainPending;
use rapier3d::{
    control::{CharacterLength, KinematicCharacterController},
    parry::query::details::ShapeCastOptions,
    prelude::{Capsule, Pose, QueryFilter, QueryPipeline, Ray},
};

use crate::{actor_control::angle_between, Actor, ActorIntent, ActorScale, CollisionSize};

/// Downward acceleration, in m/s².
const GRAVITY: f32 = 9.81;

/// How fast an actor turns toward its heading, in radians per second.
const TURN_RATE: f32 = 5.0;

/// How far up from the actor's feet the readiness ray starts, and how far down it reaches.
const GROUND_RAY_RISE: f32 = 2.0;
const GROUND_RAY_LENGTH: f32 = 1000.0;

/// Where an actor is in its realm's physics, from the fixed ticks. The actor's `Transform` is
/// drawn between the previous tick's pose and the last, so it lags the simulation by up to a
/// tick.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct ActorMotion {
    /// The feet, at the last tick.
    pub position: Vec3,
    /// The feet, at the tick before.
    pub previous: Vec3,
    /// The way the actor faces, at the last tick, as `ActorIntent::heading`.
    pub facing: f32,
    /// The way the actor faced at the tick before.
    pub previous_facing: f32,
    /// Up is positive.
    pub vertical_speed: f32,
    /// Standing on walkable ground as of the last tick: not on a slope too steep to climb.
    pub grounded: bool,
    /// The normal of that ground, while `grounded`.
    pub ground_normal: Vec3,
    /// Off the ground and, at some tick since leaving it, more than `FALL_HEIGHT` above it (R25).
    /// Shorter drops, such as a step down, do not count.
    pub falling: bool,
}

/// Marks an actor that `move_actors` skips: standing on walkable ground, facing its heading and
/// asked for no speed, so that moving it would leave it where it is. It wakes when that stops
/// being true, or when the colliders of its realm change.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct ActorResting {
    /// The realm's `RealmPhysics::collider_changes` when the actor came to rest.
    pub collider_changes: u64,
}

/// How far a facing may be from the heading for the actor to count as facing it, in radians.
const FACING_TOLERANCE: f32 = 1e-4;

fn rests(motion: &ActorMotion, intent: &ActorIntent) -> bool {
    motion.grounded
        && !motion.falling
        && intent.speed == 0.0
        && angle_between(motion.facing, intent.heading).abs() < FACING_TOLERANCE
}

/// How far above the ground an actor must be to count as falling.
const FALL_HEIGHT: f32 = 0.7;

/// Surfaces whose normal's y is under this are walls (R24).
const WALKABLE_NORMAL_Y: f32 = 0.7;

/// The gap the controller keeps from what it touches, and how far down the walkable-ground cast
/// reaches beyond it.
const GROUND_CAST_GAP: f32 = 0.01;
const GROUND_CAST_LENGTH: f32 = 0.06;

fn controller() -> KinematicCharacterController {
    KinematicCharacterController {
        offset: CharacterLength::Absolute(GROUND_CAST_GAP),
        max_slope_climb_angle: WALKABLE_NORMAL_Y.acos(),
        min_slope_slide_angle: WALKABLE_NORMAL_Y.acos(),
        // R25.
        snap_to_ground: Some(CharacterLength::Absolute(0.1)),
        ..default()
    }
}

/// The actor's capsule, centered on the origin.
pub(crate) fn capsule(size: Option<&CollisionSize>, scale: Option<&ActorScale>) -> Capsule {
    let size = size.cloned().unwrap_or_default().scaled_by(scale);
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
        let facing = transform.rotation.to_euler(EulerRot::YXZ).0;
        let ray = Ray::new(position + Vec3::Y * GROUND_RAY_RISE, Vec3::NEG_Y);
        let ground =
            physics
                .world()
                .cast_ray(&ray, GROUND_RAY_LENGTH, true, QueryFilter::default());
        if ground.is_some() {
            commands.entity(entity).insert(ActorMotion {
                position,
                previous: position,
                facing,
                previous_facing: facing,
                vertical_speed: 0.0,
                grounded: false,
                ground_normal: Vec3::Y,
                falling: false,
            });
        }
    }
}

/// Moves actors one fixed tick: turning toward their intent's heading, travelling along their
/// facing, falling under gravity, stopped and slid by what they meet, and pushed out of what they
/// are inside (R26). Runs after the realm steps, so colliders inserted since the last tick are in
/// its broad phase. Only the viewpoint's realm steps, so actors in other realms hold still.
/// Resting actors (`ActorResting`) are skipped.
pub(crate) fn move_actors(
    mut commands: Commands,
    mut actors: Query<(
        Entity,
        &Actor,
        &mut ActorMotion,
        Option<&ActorIntent>,
        Option<&CollisionSize>,
        Option<&ActorScale>,
        Option<&ActorResting>,
    )>,
    realms: Query<&RealmPhysics>,
    viewpoint: Res<Viewpoint>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.timestep().as_secs_f32();
    let controller = controller();
    for (entity, actor, mut motion, intent, size, scale, resting) in &mut actors {
        motion.previous = motion.position;
        motion.previous_facing = motion.facing;
        if viewpoint.realm != Some(actor.realm) {
            continue;
        }
        let Ok(physics) = realms.get(actor.realm) else {
            continue;
        };
        let intent = intent.copied().unwrap_or_default();
        let still_resting =
            resting.is_some_and(|resting| resting.collider_changes == physics.collider_changes());
        if still_resting && rests(&motion, &intent) {
            continue;
        }

        let shape = capsule(size, scale);
        let center = motion.position + Vec3::Y * (shape.half_height() + shape.radius);
        let turn = angle_between(motion.facing, intent.heading);
        motion.facing = (motion.facing + turn.clamp(-TURN_RATE * dt, TURN_RATE * dt))
            .rem_euclid(std::f32::consts::TAU);
        let mut travel = Quat::from_rotation_y(motion.facing) * Vec3::Z * intent.speed;
        let queries = physics.world().query_pipeline();
        // On a slope, travel along the ground at the full horizontal pace. Left to the controller,
        // travel into a slope is projected onto it, which slows the climb but not the sideways
        // part, so actors drift off the sides of ramps and stairs.
        if motion.grounded {
            let normal = motion.ground_normal;
            travel.y = -(normal.x * travel.x + normal.z * travel.z) / normal.y;
        }
        // A grounded actor standing still asks for no movement, which is when the controller
        // depenetrates.
        if !motion.grounded {
            motion.vertical_speed -= GRAVITY * dt;
        }
        let movement = controller.move_shape(
            dt,
            &queries,
            &shape,
            &Pose::from_translation(center),
            (travel + Vec3::Y * motion.vertical_speed) * dt,
            |_| {},
        );
        motion.position += movement.translation;
        // The controller's `grounded` includes steep slopes, where the actor must keep falling to
        // slide down (R24). From the pose the next tick starts at, so it serves that tick too.
        let ground = movement
            .grounded
            .then(|| walkable_ground(&queries, &shape, center + movement.translation))
            .flatten();
        motion.grounded = ground.is_some();
        if let Some(normal) = ground {
            motion.ground_normal = normal;
            motion.vertical_speed = 0.0;
            motion.falling = false;
        } else if !motion.falling {
            // A ray from just above the feet, so it starts outside the ground the controller
            // keeps its gap from.
            let ray = Ray::new(motion.position + Vec3::Y * GROUND_CAST_GAP, Vec3::NEG_Y);
            motion.falling = queries
                .cast_ray(&ray, FALL_HEIGHT + GROUND_CAST_GAP, true)
                .is_none();
        }
        if rests(&motion, &intent) {
            commands.entity(entity).insert(ActorResting {
                collider_changes: physics.collider_changes(),
            });
        } else if resting.is_some() {
            commands.entity(entity).remove::<ActorResting>();
        }
    }
}

/// The normal of the ground just under a capsule centered at `center`, if it is gentle enough to
/// walk on.
fn walkable_ground(queries: &QueryPipeline, shape: &Capsule, center: Vec3) -> Option<Vec3> {
    let (_, hit) = queries.cast_shape(
        &Pose::from_translation(center),
        Vec3::NEG_Y,
        shape,
        ShapeCastOptions {
            target_distance: GROUND_CAST_GAP,
            stop_at_penetration: false,
            max_time_of_impact: GROUND_CAST_LENGTH,
            compute_impact_geometry_on_penetration: true,
        },
    )?;
    (hit.normal1.y >= WALKABLE_NORMAL_Y).then_some(hit.normal1)
}

/// Draws each actor between its last two ticks' poses.
pub(crate) fn interpolate_actors(
    mut actors: Query<(&ActorMotion, &mut Transform)>,
    time: Res<Time<Fixed>>,
) {
    let fraction = time.overstep_fraction();
    for (motion, mut transform) in &mut actors {
        transform.translation = motion.previous.lerp(motion.position, fraction);
        let turn = angle_between(motion.previous_facing, motion.facing);
        transform.rotation = Quat::from_rotation_y(motion.previous_facing + turn * fraction);
    }
}
