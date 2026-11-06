//! The interaction probe: E casts a ray from the player's actor along its facing and logs what it
//! hits. TODO(PAN21): the hit's interactions.

use bevy::prelude::*;
use bevy_action_map::prelude::*;
use panoply_core::{realm_physics::collision_groups, RealmPhysics};
use panoply_scenery::scenery_element::SceneryElement;
use rapier3d::prelude::{Group, InteractionGroups, InteractionTestMode, QueryFilter, Ray};

use crate::{
    actor_control::{PlayerControl, PlayerInteract},
    actor_physics::capsule,
    Actor, ActorMotion, ActorScale, CollisionSize, PlayerActor,
};

/// How far the probe reaches from the actor's axis: the default interaction distance (R92).
const PROBE_REACH: f32 = 1.5;

/// Logs the first collider within reach ahead of the player's actor, from the middle of its
/// capsule. The canopy is passed through (D39), and actors are not yet colliders (D41).
pub(crate) fn probe_interaction(
    input: ContextActions<PlayerControl>,
    players: Query<
        (
            &Actor,
            &ActorMotion,
            Option<&CollisionSize>,
            Option<&ActorScale>,
        ),
        With<PlayerActor>,
    >,
    realms: Query<&RealmPhysics>,
    names: Query<(Option<&Name>, Option<&SceneryElement>)>,
) {
    if !input.fired::<PlayerInteract>() {
        return;
    }
    let Ok((actor, motion, size, scale)) = players.single() else {
        return;
    };
    let Ok(physics) = realms.get(actor.realm) else {
        return;
    };
    let shape = capsule(size, scale);
    let origin = motion.position + Vec3::Y * (shape.half_height() + shape.radius);
    let ray = Ray::new(origin, Quat::from_rotation_y(motion.facing) * Vec3::Z);
    let filter = QueryFilter::default().groups(InteractionGroups::new(
        Group::ALL,
        !collision_groups::CANOPY,
        InteractionTestMode::And,
    ));
    let world = physics.world();
    let Some((handle, distance)) = world.cast_ray(&ray, PROBE_REACH, true, filter) else {
        info!("Probe: nothing within {PROBE_REACH} m");
        return;
    };
    let collider = &world.colliders[handle];
    let memberships = collider.collision_groups().memberships;
    let group = collision_groups::CAPTIONED
        .iter()
        .find(|(_, group)| memberships.intersects(*group))
        .map_or("?", |(caption, _)| caption);
    let owner = Entity::try_from_bits(collider.user_data as u64);
    let label = match owner.and_then(|owner| names.get(owner).ok()) {
        Some((_, Some(element))) => element
            .exemplar
            .path()
            .map_or_else(|| "an unnamed exemplar".to_owned(), ToString::to_string),
        Some((Some(name), None)) => name.to_string(),
        _ => "an unnamed entity".to_owned(),
    };
    info!("Probe: {group}, {label}, at {distance:.2} m");
}
