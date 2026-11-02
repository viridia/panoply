use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use rapier3d::prelude::*;

use crate::{Realm, Viewpoint};

/// Collision groups, as rapier membership and filter bits. Each is defined when its kind of
/// collider arrives, after the three.js version's nine (see `docs/faery.md`, Physics).
pub mod collision_groups {
    use rapier3d::prelude::Group;

    /// Parcel ground.
    pub const TERRAIN: Group = Group::GROUP_1;

    /// Precinct floor regions.
    pub const FLOOR: Group = Group::GROUP_2;

    /// Wall tiles.
    pub const WALL: Group = Group::GROUP_3;

    /// Fixtures.
    pub const FIXTURE: Group = Group::GROUP_4;

    /// Generated tree trunks.
    pub const FLORA: Group = Group::GROUP_5;

    /// The surface over generated forests that catches what falls onto them.
    pub const CANOPY: Group = Group::GROUP_6;

    /// Actors. Their capsules are not yet colliders in the world (D41), so this only names them
    /// in the debug view.
    pub const ACTOR: Group = Group::GROUP_7;

    /// The groups of a parcel's colliders, as opposed to a precinct's.
    pub const PARCEL: Group = TERRAIN.union(FLORA).union(CANOPY);

    /// Every group, with its caption in the editor's Show menu.
    pub const CAPTIONED: &[(&str, Group)] = &[
        ("Terrain", TERRAIN),
        ("Floors", FLOOR),
        ("Walls", WALL),
        ("Fixtures", FIXTURE),
        ("Trees", FLORA),
        ("Canopy", CANOPY),
        ("Actors", ACTOR),
    ];
}

/// Colliders an entity owns in its realm's physics world, removed from it when this component
/// is replaced or removed, or the entity despawns. Immutable, so every change goes through
/// insertion and the hook.
#[derive(Component, Debug)]
#[component(immutable, on_discard = discard_owned_colliders)]
pub struct OwnedColliders {
    pub realm: Entity,
    pub handles: Vec<ColliderHandle>,
}

fn discard_owned_colliders(mut world: DeferredWorld, context: HookContext) {
    let owned = world.get::<OwnedColliders>(context.entity).unwrap();
    let (realm, handles) = (owned.realm, owned.handles.clone());
    // Handles are generational: one already removed with its body is skipped.
    if let Some(mut physics) = world.get_mut::<RealmPhysics>(realm) {
        for handle in handles {
            physics.remove_collider(handle);
        }
    }
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
