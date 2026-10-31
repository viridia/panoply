use bevy::prelude::*;
use rapier3d::prelude::*;

use crate::{Realm, Viewpoint};

/// Collision groups, as rapier membership and filter bits. Each is defined when its kind of
/// collider arrives, after the three.js version's nine (see `docs/faery.md`, Physics).
pub mod collision_groups {
    use rapier3d::prelude::Group;

    /// Parcel ground.
    pub const TERRAIN: Group = Group::GROUP_1;
}

/// One rapier world per realm, so realms seen through portals never collide with each other.
#[derive(Component, Default)]
pub struct RealmPhysics {
    world: PhysicsWorld,
}

impl RealmPhysics {
    /// Advances the simulation by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        self.world.integration_parameters.dt = dt;
        self.world.step();
    }

    /// The rapier world, for queries and debug rendering.
    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    /// Number of colliders in this realm's physics world.
    pub fn collider_count(&self) -> usize {
        self.world.colliders.len()
    }

    pub fn insert_body(&mut self, body: RigidBody) -> RigidBodyHandle {
        self.world.insert_body(body)
    }

    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.world.remove_body(handle);
    }

    pub fn insert_collider(
        &mut self,
        handle: RigidBodyHandle,
        collider: Collider,
    ) -> ColliderHandle {
        self.world.insert_collider(collider, Some(handle))
    }

    pub fn remove_collider(&mut self, collider: ColliderHandle) {
        self.world.remove_collider(collider);
    }
}

/// Steps the viewpoint's realm by the fixed timestep. Other realms are not stepped.
pub fn realm_physics_system(
    mut realm_query: Query<(&Realm, &mut RealmPhysics)>,
    viewpoint: Res<Viewpoint>,
    time: Res<Time<Fixed>>,
) {
    let Some(realm_entity) = viewpoint.realm else {
        return;
    };

    if let Ok((_realm, mut realm_physics)) = realm_query.get_mut(realm_entity) {
        realm_physics.step(time.timestep().as_secs_f32());
    }
}
