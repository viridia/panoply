use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use rapier3d::prelude::*;

use crate::{Realm, ToRapier, Viewpoint};

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

    /// Scenery's `marker` colliders, on things the editor shows but the player never sees: chalk
    /// marks on the stage, such as patrol paths, music cues, the sensors of traps and quests, and
    /// the tethers of wandering. The collider is only the editor's handle for picking one; the game
    /// reads the marks themselves, and no game query meets the collider.
    pub const MARKER: Group = Group::GROUP_8;

    /// What the game's own casts and movement meet: every group but the editor's markers, and the
    /// actors, which are not colliders (D41).
    pub const BLOCKING: Group = Group::ALL.difference(MARKER).difference(ACTOR);

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
        ("Markers", MARKER),
    ];
}

/// A query filter meeting only colliders in `groups`.
pub fn group_filter(groups: Group) -> QueryFilter<'static> {
    QueryFilter::default().groups(InteractionGroups::new(
        Group::ALL,
        groups,
        InteractionTestMode::And,
    ))
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
    collider_changes: u64,
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

    /// The first collider in `groups` along `ray`, within `range`: the entity its `user_data`
    /// names, and how far along the ray it was hit. Game and editor picking both cast this, each
    /// with its own groups, so the ray stops at whatever of those it meets first.
    pub fn pick(&self, ray: Ray3d, range: f32, groups: Group) -> Option<(Entity, f32)> {
        let ray = Ray::new(ray.origin.to_rapier(), ray.direction.to_rapier());
        let (handle, distance) = self
            .world
            .cast_ray(&ray, range, true, group_filter(groups))?;
        let collider = self.world.colliders.get(handle)?;
        Some((Entity::try_from_bits(collider.user_data as u64)?, distance))
    }

    /// Number of colliders in this realm's physics world.
    pub fn collider_count(&self) -> usize {
        self.world.colliders.len()
    }

    /// Counts insertions and removals of colliders, so what depends on the world staying as it
    /// was, such as a resting actor, can tell when it changed.
    pub fn collider_changes(&self) -> u64 {
        self.collider_changes
    }

    pub fn insert_body(&mut self, body: RigidBody) -> RigidBodyHandle {
        self.world.insert_body(body)
    }

    /// Removes the body and its colliders.
    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.world.remove_body(handle);
        self.collider_changes += 1;
    }

    pub fn insert_collider(
        &mut self,
        handle: RigidBodyHandle,
        collider: Collider,
    ) -> ColliderHandle {
        self.collider_changes += 1;
        self.world.insert_collider(collider, Some(handle))
    }

    pub fn remove_collider(&mut self, collider: ColliderHandle) {
        self.world.remove_collider(collider);
        self.collider_changes += 1;
    }
}

/// Most fixed steps a frame catches up (D68). A frame that took longer than this many steps feeds
/// only this many into the accumulator, and the rest of its time is dropped: under overload the
/// game slows down rather than freezing, as it does when each frame owes more steps than the last.
pub const MAX_CATCH_UP_STEPS: u32 = 4;

/// Caps each frame's virtual time at [`MAX_CATCH_UP_STEPS`] fixed steps, in place of Bevy's 250 ms
/// (16 steps at 64 Hz).
pub fn cap_catch_up(fixed: Res<Time<Fixed>>, mut virt: ResMut<Time<Virtual>>) {
    virt.set_max_delta(fixed.timestep() * MAX_CATCH_UP_STEPS);
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
