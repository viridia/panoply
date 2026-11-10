//! Actors' movement through their realm's physics: a capsule moved by rapier's kinematic
//! character controller each fixed tick, and drawn between the last two ticks' poses.
//!
//! The capsule is a query shape, not a collider in the world, so actors do not yet collide with
//! each other.

use bevy::{ecs::system::SystemParam, prelude::*};
use panoply_core::{RealmPhysics, ToBevy, ToRapier, Viewpoint};
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
    /// The feet, at the last tick the actor was `grounded`: where it goes back to if it falls out
    /// of the world (R27).
    pub last_stood: Vec3,
}

/// How far below where it last stood an actor falling with nothing under it goes before it is put
/// back there, in m: far enough that the fall shows.
const FALL_OUT_DEPTH: f32 = 2.0;

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

/// How far past its capsule an actor's footprint reaches, in meters: further than it moves in a
/// tick, so the ground ahead is built before the actor steps onto it.
const FOOTPRINT_MARGIN: f32 = 0.5;

/// Whether the colliders under an actor are built. The parcel and precinct caches follow the
/// view, and evict what they drop with its colliders, so an actor away from the view can lose the
/// ground under it; one moved then would fall through where it stood.
#[derive(SystemParam)]
pub(crate) struct GroundReady<'w, 's> {
    terrain: TerrainPending<'w, 's>,
    scenery: SceneryPending<'w, 's>,
}

impl GroundReady<'_, '_> {
    /// Whether everything under the footprint of a capsule of `radius` at `position` in `realm`
    /// is built.
    fn under(&self, realm: Entity, position: Vec3, radius: f32) -> bool {
        let reach = Vec2::splat(radius + FOOTPRINT_MARGIN);
        let footprint = Rect::from_corners(position.xz() - reach, position.xz() + reach);
        self.terrain.area_waiting(realm, footprint).is_none()
            && self.scenery.area_waiting(realm, footprint).is_none()
    }
}

/// Gives an actor physics once the ground under it is built: its footprint (`GroundReady`), and a
/// ray down from above the actor hits a collider, which it does only after the realm has stepped
/// since the collider was inserted.
pub(crate) fn start_actor_physics(
    mut commands: Commands,
    actors: Query<
        (
            Entity,
            &Actor,
            &Transform,
            Option<&CollisionSize>,
            Option<&ActorScale>,
        ),
        Without<ActorMotion>,
    >,
    realms: Query<&RealmPhysics>,
    ground: GroundReady,
) {
    for (entity, actor, transform, size, scale) in &actors {
        let Ok(physics) = realms.get(actor.realm) else {
            continue;
        };
        let position = transform.translation;
        if !ground.under(actor.realm, position, capsule(size, scale).radius) {
            continue;
        }
        let facing = transform.rotation.to_euler(EulerRot::YXZ).0;
        let ray = down_ray(position + Vec3::Y * GROUND_RAY_RISE);
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
                // Over ground, as the ray found.
                last_stood: position,
            });
        }
    }
}

/// Moves actors one fixed tick: turning toward their intent's heading, travelling along their
/// facing, falling under gravity, stopped and slid by what they meet, and pushed out of what they
/// are inside (R26), and put back where they last stood when they fall out of the world (R27).
/// Runs after the realm steps, so colliders inserted since the last tick are in its broad phase.
/// Only the viewpoint's realm steps, so actors in other realms hold still, and so do actors whose
/// ground is not built (`GroundReady`), mid-fall included, until it is. Resting actors
/// (`ActorResting`) are skipped.
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
    ground: GroundReady,
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
        if !ground.under(actor.realm, motion.position, shape.radius) {
            continue;
        }
        let queries = physics.world().query_pipeline();
        let mut center = motion.position + Vec3::Y * (shape.half_height() + shape.radius);
        let pushed = push_out(&controller, &queries, &shape, center, dt);
        motion.position += pushed;
        center += pushed;
        let turn = angle_between(motion.facing, intent.heading);
        motion.facing = (motion.facing + turn.clamp(-TURN_RATE * dt, TURN_RATE * dt))
            .rem_euclid(std::f32::consts::TAU);
        let mut travel = Quat::from_rotation_y(motion.facing) * Vec3::Z * intent.speed;
        // On a slope, travel along the ground at the full horizontal pace. Left to the controller,
        // travel into a slope is projected onto it, which slows the climb but not the sideways
        // part, so actors drift off the sides of ramps and stairs.
        if motion.grounded {
            let normal = motion.ground_normal;
            travel.y = -(normal.x * travel.x + normal.z * travel.z) / normal.y;
        }
        if !motion.grounded {
            motion.vertical_speed -= GRAVITY * dt;
        }
        let movement = controller.move_shape(
            dt,
            &queries,
            &shape,
            &Pose::from_translation(center.to_rapier()),
            ((travel + Vec3::Y * motion.vertical_speed) * dt).to_rapier(),
            |_| {},
        );
        let moved = movement.translation.to_bevy();
        motion.position += moved;
        // The controller's `grounded` includes steep slopes, where the actor must keep falling to
        // slide down (R24). From the pose the next tick starts at, so it serves that tick too.
        let ground = movement
            .grounded
            .then(|| walkable_ground(&queries, &shape, center + moved))
            .flatten();
        motion.grounded = ground.is_some();
        if let Some(normal) = ground {
            motion.ground_normal = normal;
            motion.vertical_speed = 0.0;
            motion.falling = false;
            motion.last_stood = motion.position;
        } else if !motion.falling {
            // A ray from just above the feet, so it starts outside the ground the controller
            // keeps its gap from.
            let ray = down_ray(motion.position + Vec3::Y * GROUND_CAST_GAP);
            motion.falling = queries
                .cast_ray(&ray, FALL_HEIGHT + GROUND_CAST_GAP, true)
                .is_none();
        } else if motion.position.y < motion.last_stood.y - FALL_OUT_DEPTH {
            // Nothing under it: off the map's edge or through a hole, out of the world (R27).
            let ray = down_ray(motion.position);
            if queries.cast_ray(&ray, GROUND_RAY_LENGTH, true).is_none() {
                motion.position = motion.last_stood;
                motion.previous = motion.last_stood;
                motion.vertical_speed = 0.0;
                motion.falling = false;
            }
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

/// The translation that pushes a capsule centered at `center` out of what it is inside (R26), or
/// zero if it is inside nothing. The controller pushes out only when asked for no movement, so an
/// actor that falls or walks would otherwise stay inside ground that was built around it, such as
/// a parcel's collider inserted mid-fall, and every move would cost a full set of its shape-casts
/// while moving nothing. Each call corrects up to a quarter of the capsule's height, so deeper
/// overlaps take a few ticks. A standing actor rests the controller's gap clear of the ground, so
/// it overlaps nothing and is not moved.
fn push_out(
    controller: &KinematicCharacterController,
    queries: &QueryPipeline,
    shape: &Capsule,
    center: Vec3,
    dt: f32,
) -> Vec3 {
    let pose = Pose::from_translation(center.to_rapier());
    if queries.intersect_shape(pose, shape).next().is_none() {
        return Vec3::ZERO;
    }
    controller
        .move_shape(dt, queries, shape, &pose, Vec3::ZERO.to_rapier(), |_| {})
        .translation
        .to_bevy()
}

/// A ray straight down from `origin`.
fn down_ray(origin: Vec3) -> Ray {
    Ray::new(origin.to_rapier(), Vec3::NEG_Y.to_rapier())
}

/// The normal of the ground just under a capsule centered at `center`, if it is gentle enough to
/// walk on.
fn walkable_ground(queries: &QueryPipeline, shape: &Capsule, center: Vec3) -> Option<Vec3> {
    let (_, hit) = queries.cast_shape(
        &Pose::from_translation(center.to_rapier()),
        Vec3::NEG_Y.to_rapier(),
        shape,
        ShapeCastOptions {
            target_distance: GROUND_CAST_GAP,
            stop_at_penetration: false,
            max_time_of_impact: GROUND_CAST_LENGTH,
            compute_impact_geometry_on_penetration: true,
        },
    )?;
    (hit.normal1.y >= WALKABLE_NORMAL_Y).then_some(hit.normal1.to_bevy())
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

#[cfg(test)]
mod tests {
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    use super::*;

    const DT: f32 = 1.0 / 64.0;

    /// A flat parcel's ground at y = 0: 16 m, 64 by 64 cells, as `ground_mesh.rs` builds it.
    fn parcel_ground() -> RealmPhysics {
        let (size, cells) = (16.0, 64u32);
        let step = size / cells as f32;
        let vertices = (0..=cells)
            .flat_map(|z| (0..=cells).map(move |x| Vec3::new(x as f32, 0.0, z as f32) * step))
            .map(ToRapier::to_rapier)
            .collect();
        let row = cells + 1;
        let indices = (0..cells)
            .flat_map(|z| (0..cells).map(move |x| (z, x)))
            .flat_map(|(z, x)| {
                let (a, b) = (z * row + x, z * row + x + 1);
                let (c, d) = ((z + 1) * row + x + 1, (z + 1) * row + x);
                [[a, b, d], [b, c, d]]
            })
            .collect();
        let mut physics = RealmPhysics::default();
        let body = physics.insert_body(RigidBodyBuilder::fixed().build());
        physics.insert_collider(body, ColliderBuilder::trimesh(vertices, indices).unwrap().build());
        // The query pipeline takes in new colliders when the world steps.
        physics.step(DT);
        physics
    }

    /// Where a capsule's center is when its feet are at `feet` over the middle of the parcel.
    fn center_at(shape: &Capsule, feet: f32) -> Vec3 {
        Vec3::new(8.0, feet + shape.half_height() + shape.radius, 8.0)
    }

    #[test]
    fn a_falling_actor_inside_the_ground_is_pushed_out_and_lands() {
        let physics = parcel_ground();
        let queries = physics.world().query_pipeline();
        let (controller, shape) = (controller(), capsule(None, None));
        let mut center = center_at(&shape, -0.4);
        let mut vertical_speed = 0.0;
        let mut landed = false;
        for _ in 0..4 {
            center += push_out(&controller, &queries, &shape, center, DT);
            vertical_speed -= GRAVITY * DT;
            let movement = controller.move_shape(
                DT,
                &queries,
                &shape,
                &Pose::from_translation(center.to_rapier()),
                (Vec3::Y * vertical_speed * DT).to_rapier(),
                |_| {},
            );
            center += movement.translation.to_bevy();
            landed = movement.grounded;
        }
        let feet = center.y - shape.half_height() - shape.radius;
        assert!(landed, "not grounded, feet at {feet}");
        assert!((0.0..0.05).contains(&feet), "feet at {feet}, not on the ground");
        let pose = Pose::from_translation(center.to_rapier());
        assert!(queries.intersect_shape(pose, &shape).next().is_none(), "still inside the ground");
    }

    #[test]
    fn a_standing_actor_is_not_pushed() {
        let physics = parcel_ground();
        let queries = physics.world().query_pipeline();
        let (controller, shape) = (controller(), capsule(None, None));
        let center = center_at(&shape, GROUND_CAST_GAP);
        assert_eq!(push_out(&controller, &queries, &shape, center, DT), Vec3::ZERO);
    }
}
