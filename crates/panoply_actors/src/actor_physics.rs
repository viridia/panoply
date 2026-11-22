//! Actors' movement through their realm's physics: a capsule moved by rapier's kinematic
//! character controller each fixed tick, and drawn between the last two ticks' poses.
//!
//! The same capsule is a collider in the `ACTOR` group, so a ray or the probe can hit the actor
//! (D95). Actors' movement leaves that group out, so actors never block each other (R86).

use bevy::{ecs::system::SystemParam, prelude::*};
use panoply_core::{
    realm_physics::{collision_groups, group_filter},
    OwnedColliders, RealmPhysics, ToBevy, ToRapier, Viewpoint,
};
use panoply_scenery::SceneryPending;
use panoply_terrain::TerrainPending;
use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    parry::query::details::ShapeCastOptions,
    prelude::{
        Capsule, Collider, ColliderBuilder, Group, InteractionGroups, InteractionTestMode, Pose,
        QueryFilter, QueryPipeline, Ray, SharedShape,
    },
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
    /// The velocity other actors push it at, on the ground plane, in m/s (`bump_actors`).
    pub push: Vec2,
}

/// How far below where it last stood an actor falling with nothing under it goes before it is put
/// back there, in m: far enough that the fall shows.
const FALL_OUT_DEPTH: f32 = 2.0;

/// Marks an actor that `move_actors` skips: standing on walkable ground, facing its heading, asked
/// for no speed and pushed by no one, so that moving it would leave it where it is. It wakes when that stops
/// being true, or when the colliders of its realm change.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct ActorResting {
    /// The realm's `RealmPhysics::collider_changes` when the actor came to rest.
    pub collider_changes: u64,
}

/// Marks an actor whose `ActorMotion` something other than physics sets, as the editor sets its
/// preview of an actor about to be placed: `move_actors` leaves it where it is put, and it is
/// drawn and animated from its motion as any actor is. It is given its motion when spawned, since
/// `start_actor_physics` would otherwise give it one, to move.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
pub struct ActorHeld;

/// How far a facing may be from the heading for the actor to count as facing it, in radians.
const FACING_TOLERANCE: f32 = 1e-4;

fn rests(motion: &ActorMotion, intent: &ActorIntent) -> bool {
    motion.grounded
        && !motion.falling
        && intent.speed == 0.0
        && motion.push == Vec2::ZERO
        && angle_between(motion.facing, intent.heading).abs() < FACING_TOLERANCE
}

/// How far above the ground an actor must be to count as falling.
const FALL_HEIGHT: f32 = 0.7;

/// Surfaces whose normal's y is under this are walls (R24).
pub(crate) const WALKABLE_NORMAL_Y: f32 = 0.7;

/// The gap the controller keeps from what it touches, and how far down the walkable-ground cast
/// reaches beyond it.
const GROUND_CAST_GAP: f32 = 0.01;
const GROUND_CAST_LENGTH: f32 = 0.06;

/// The highest ledge an actor steps up onto as it walks (R105), in m, and the depth of free space
/// it needs on top.
const STEP_HEIGHT: f32 = 0.25;
const STEP_DEPTH: f32 = 0.1;

fn controller() -> KinematicCharacterController {
    KinematicCharacterController {
        offset: CharacterLength::Absolute(GROUND_CAST_GAP),
        max_slope_climb_angle: WALKABLE_NORMAL_Y.acos(),
        min_slope_slide_angle: WALKABLE_NORMAL_Y.acos(),
        // R105. Without it, a capsule meets a ledge's edge on its rounded bottom, at a normal too
        // steep to climb, so even a curb is a wall.
        autostep: Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(STEP_HEIGHT),
            min_width: CharacterLength::Absolute(STEP_DEPTH),
            include_dynamic_bodies: false,
        }),
        // R25.
        snap_to_ground: Some(CharacterLength::Absolute(0.1)),
        ..default()
    }
}

/// The actor's capsule, centered on the origin; an actor's feet are at its bottom.
pub fn capsule(size: Option<&CollisionSize>, scale: Option<&ActorScale>) -> Capsule {
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
        let ground = physics
            .world()
            .cast_ray(&ray, GROUND_RAY_LENGTH, true, blocking());
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
                push: Vec2::ZERO,
            });
        }
    }
}

/// Moves actors one fixed tick: turning toward their intent's heading, travelling along their
/// facing and as other actors push them (`bump_actors`), falling under gravity, stopped and slid by what they meet, and pushed out of what they
/// are inside (R26), and put back where they last stood when they fall out of the world (R27).
/// Runs after the realm steps, so colliders inserted since the last tick are in its broad phase.
/// Only the viewpoint's realm steps, so actors in other realms hold still, and so do actors whose
/// ground is not built (`GroundReady`), mid-fall included, until it is. Resting actors
/// (`ActorResting`) are skipped, and so are held ones (`ActorHeld`).
pub(crate) fn move_actors(
    mut commands: Commands,
    mut actors: Query<
        (
            Entity,
            &Actor,
            &mut ActorMotion,
            Option<&ActorIntent>,
            Option<&CollisionSize>,
            Option<&ActorScale>,
            Option<&ActorResting>,
        ),
        Without<ActorHeld>,
    >,
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
        let queries = physics.world().query_pipeline_with_filter(blocking());
        let mut center = motion.position + Vec3::Y * (shape.half_height() + shape.radius);
        let pushed = push_out(&controller, &queries, &shape, center, dt);
        motion.position += pushed;
        center += pushed;
        let turn = angle_between(motion.facing, intent.heading);
        motion.facing = (motion.facing + turn.clamp(-TURN_RATE * dt, TURN_RATE * dt))
            .rem_euclid(std::f32::consts::TAU);
        let mut travel = Quat::from_rotation_y(motion.facing) * Vec3::Z * intent.speed
            + Vec3::new(motion.push.x, 0.0, motion.push.y);
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

/// Keeps each actor's collider on its capsule: inserted once the actor has physics, inserted again
/// when its capsule's size or its realm changes, and otherwise moved to where the last tick left
/// the actor, which costs nothing for an actor that stayed put (`RealmPhysics::move_collider`). A
/// held actor has none, since it is not yet in the world. Runs after `move_actors`, so the next
/// step brings the broad phase up to date before anything casts at the actor.
pub(crate) fn place_actor_colliders(
    mut commands: Commands,
    actors: Query<
        (
            Entity,
            &Actor,
            &ActorMotion,
            Option<&CollisionSize>,
            Option<&ActorScale>,
            Option<&OwnedColliders>,
        ),
        Without<ActorHeld>,
    >,
    mut realms: Query<&mut RealmPhysics>,
) {
    for (entity, actor, motion, size, scale, owned) in &actors {
        let Ok(mut physics) = realms.get_mut(actor.realm) else {
            continue;
        };
        let shape = capsule(size, scale);
        let center = motion.position + Vec3::Y * (shape.half_height() + shape.radius);
        let current = owned
            .filter(|owned| owned.realm == actor.realm)
            .and_then(|owned| owned.handles.first().copied())
            .filter(|&handle| {
                physics
                    .world()
                    .colliders
                    .get(handle)
                    .and_then(|collider| collider.shape().as_capsule())
                    .is_some_and(|built| {
                        built.radius == shape.radius && built.half_height() == shape.half_height()
                    })
            });
        if let Some(handle) = current {
            physics.move_collider(handle, center);
            continue;
        }
        let collider = actor_collider(shape, center, entity);
        // Replacing the component removes the old collider (`OwnedColliders`' hook).
        commands.entity(entity).insert(OwnedColliders {
            realm: actor.realm,
            handles: vec![physics.insert_standalone_collider(collider)],
        });
    }
}

/// An actor's collider: its capsule centered at `center`, in the `ACTOR` group, naming `entity`.
/// Its filter is every group, since a query meets only colliders whose filter has the query's
/// groups; it is kept out of contacts by having no body (`insert_standalone_collider`).
fn actor_collider(shape: Capsule, center: Vec3, entity: Entity) -> Collider {
    ColliderBuilder::new(SharedShape::new(shape))
        .translation(center.to_rapier())
        .collision_groups(InteractionGroups::new(
            collision_groups::ACTOR,
            Group::ALL,
            InteractionTestMode::And,
        ))
        .user_data(entity.to_bits() as u128)
        .build()
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

/// What an actor meets: everything but the editor's markers.
fn blocking() -> QueryFilter<'static> {
    group_filter(collision_groups::BLOCKING)
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
    use rapier3d::prelude::{
        ColliderBuilder, Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder,
    };

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
        physics.insert_collider(
            body,
            ColliderBuilder::trimesh(vertices, indices).unwrap().build(),
        );
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
        assert!(
            (0.0..0.05).contains(&feet),
            "feet at {feet}, not on the ground"
        );
        let pose = Pose::from_translation(center.to_rapier());
        assert!(
            queries.intersect_shape(pose, &shape).next().is_none(),
            "still inside the ground"
        );
    }

    #[test]
    fn an_actor_falls_through_a_marker() {
        let mut physics = parcel_ground();
        let body = physics.insert_body(RigidBodyBuilder::fixed().build());
        let marker = ColliderBuilder::cuboid(1.0, 0.5, 1.0)
            .translation(Vec3::new(8.0, 0.5, 8.0).to_rapier())
            .collision_groups(InteractionGroups::new(
                collision_groups::MARKER,
                Group::ALL,
                InteractionTestMode::And,
            ))
            .build();
        physics.insert_collider(body, marker);
        physics.step(DT);
        let queries = physics.world().query_pipeline_with_filter(blocking());
        let (controller, shape) = (controller(), capsule(None, None));
        let mut center = center_at(&shape, 2.0);
        let mut vertical_speed = 0.0;
        for _ in 0..64 {
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
            if movement.grounded {
                vertical_speed = 0.0;
            }
        }
        let feet = center.y - shape.half_height() - shape.radius;
        assert!(
            (0.0..0.05).contains(&feet),
            "feet at {feet}, not on the ground under the marker"
        );
    }

    /// An actor's collider is met where it was moved once the world steps, by a pick and not by
    /// actors' movement, and neither its insertion nor its moves count as a change to the world.
    #[test]
    fn an_actor_collider_follows_its_actor_and_blocks_nothing() {
        let mut physics = parcel_ground();
        let changes = physics.collider_changes();
        let shape = capsule(None, None);
        let entity = Entity::from_raw_u32(7).unwrap();
        let (from, to) = (
            center_at(&shape, 0.0),
            center_at(&shape, 0.0) + Vec3::X * 3.0,
        );
        let handle = physics.insert_standalone_collider(actor_collider(shape, from, entity));
        physics.step(DT);
        physics.move_collider(handle, to);
        physics.step(DT);
        let down = |at: Vec3| Ray3d::new(at + Vec3::Y * 10.0, Dir3::NEG_Y);
        let groups = collision_groups::ACTOR;
        assert_eq!(
            physics
                .pick(down(to), 20.0, groups, |_| true)
                .map(|hit| hit.0),
            Some(entity)
        );
        assert_eq!(physics.pick(down(from), 20.0, groups, |_| true), None);
        assert_eq!(physics.pick(down(to), 20.0, groups, |_| false), None);
        let (blocker, _) = physics
            .world()
            .cast_ray(&down_ray(to + Vec3::Y * 10.0), 20.0, true, blocking())
            .expect("the ground under the actor");
        assert_ne!(blocker, handle);
        assert_eq!(physics.collider_changes(), changes);
    }

    #[test]
    fn a_standing_actor_is_not_pushed() {
        let physics = parcel_ground();
        let queries = physics.world().query_pipeline();
        let (controller, shape) = (controller(), capsule(None, None));
        let center = center_at(&shape, GROUND_CAST_GAP);
        assert_eq!(
            push_out(&controller, &queries, &shape, center, DT),
            Vec3::ZERO
        );
    }
}
