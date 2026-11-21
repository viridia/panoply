use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use rapier3d::prelude::*;

use crate::{Realm, ToBevy, ToRapier, Viewpoint};

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

    /// Actors' capsules (D95), which casts meet so that picking and the probe can hit an actor, and
    /// which the actors' own movement leaves out, so actors never block each other (R86).
    pub const ACTOR: Group = Group::GROUP_7;

    /// Scenery's `marker` colliders, on things the editor shows but the player never sees: chalk
    /// marks on the stage, such as patrol paths, music cues, the sensors of traps and quests, and
    /// the tethers of wandering. The collider is only the editor's handle for picking one; the game
    /// reads the marks themselves, and no game query meets the collider.
    pub const MARKER: Group = Group::GROUP_8;

    /// What actors' movement meets, and what the world is built of: every group but the editor's
    /// markers and the actors.
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

/// The entity a collider's `user_data` names.
pub fn owner(collider: &Collider) -> Option<Entity> {
    Entity::try_from_bits(collider.user_data as u64)
}

/// Whether `entity` is drawn, for [`RealmPhysics::pick`]: picking passes through what is hidden,
/// such as cutaways (R15), whatever hides it. An entity with no visibility counts as shown.
pub fn owner_shown<'a, 'w, 's, 'v>(
    visibility: &'a Query<'w, 's, &'v InheritedVisibility>,
) -> impl Fn(Entity) -> bool + use<'a, 'w, 's, 'v> {
    |entity| visibility.get(entity).ok().is_none_or(|shown| shown.get())
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

    /// The first collider in `groups` along `ray`, within `range`, whose owner `shown` accepts:
    /// the entity its `user_data` names, and how far along the ray it was hit. Game and editor
    /// picking both cast this, each with its own groups, so the ray stops at whatever of those it
    /// meets first, passing through what is hidden ([`owner_shown`]).
    pub fn pick(
        &self,
        ray: Ray3d,
        range: f32,
        groups: Group,
        shown: impl Fn(Entity) -> bool,
    ) -> Option<(Entity, f32)> {
        let ray = Ray::new(ray.origin.to_rapier(), ray.direction.to_rapier());
        let predicate = |_, collider: &Collider| owner(collider).is_none_or(&shown);
        let filter = group_filter(groups).predicate(&predicate);
        let (handle, distance) = self.world.cast_ray(&ray, range, true, filter)?;
        Some((owner(self.world.colliders.get(handle)?)?, distance))
    }

    /// As [`Self::pick`], with the normal of the surface where the ray hit it.
    pub fn pick_surface(
        &self,
        ray: Ray3d,
        range: f32,
        groups: Group,
        shown: impl Fn(Entity) -> bool,
    ) -> Option<(Entity, f32, bevy::math::Vec3)> {
        let rapier_ray = Ray::new(ray.origin.to_rapier(), ray.direction.to_rapier());
        let predicate = |_, collider: &Collider| owner(collider).is_none_or(&shown);
        let filter = group_filter(groups).predicate(&predicate);
        let (handle, hit) = self
            .world
            .cast_ray_and_get_normal(&rapier_ray, range, true, filter)?;
        let owner = owner(self.world.colliders.get(handle)?)?;
        Some((owner, hit.time_of_impact, hit.normal.to_bevy()))
    }

    /// Number of colliders in this realm's physics world.
    pub fn collider_count(&self) -> usize {
        self.world.colliders.len()
    }

    /// Counts changes to the colliders in [`collision_groups::BLOCKING`], so what depends on the
    /// world staying as it was, such as a resting actor, can tell when it changed. Actors and
    /// markers come and go without counting, since nothing stands on them.
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
        self.count_change(&collider);
        self.world.insert_collider(collider, Some(handle))
    }

    /// Inserts a collider with no body, which rapier treats as fixed: it moves only when
    /// [`Self::move_collider`] moves it, and no contacts are computed between it and other fixed
    /// colliders, which every collider here is.
    pub fn insert_standalone_collider(&mut self, collider: Collider) -> ColliderHandle {
        self.count_change(&collider);
        self.world.insert_collider(collider, None)
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        if let Some(collider) = self.world.remove_collider(handle) {
            self.count_change(&collider);
        }
    }

    /// Moves a standalone collider to `translation`, if it is elsewhere. Queries meet it there
    /// once the world has stepped, which brings the broad phase up to date. A collider left where
    /// it is costs the step nothing: rapier marks a collider for the broad phase on any mutable
    /// access, so it is compared first.
    pub fn move_collider(&mut self, handle: ColliderHandle, translation: bevy::math::Vec3) {
        let translation = translation.to_rapier();
        let colliders = &mut self.world.colliders;
        if colliders
            .get(handle)
            .is_none_or(|collider| collider.translation() == translation)
        {
            return;
        }
        let collider = &mut colliders[handle];
        collider.set_translation(translation);
        self.collider_changes += u64::from(blocks(collider));
    }

    fn count_change(&mut self, collider: &Collider) {
        self.collider_changes += u64::from(blocks(collider));
    }
}

/// Whether `collider` is one actors' movement meets.
fn blocks(collider: &Collider) -> bool {
    collider
        .collision_groups()
        .memberships
        .intersects(collision_groups::BLOCKING)
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
