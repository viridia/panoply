//! Instance keys (`docs/instances.md`, Ids), and the map from each key to the live entity that has
//! it.

use bevy::{
    asset::uuid::{self, Uuid},
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    platform::collections::{hash_map::Entry, HashMap},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// An instance's key: generated once, by the editor, and never changed. A precinct file holds it as
/// 16 bytes, JSON and the screen in its hyphenated form. On a live instance, as a component, it
/// keeps [`KeyedInstances`].
#[derive(
    Component, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize,
)]
#[component(immutable, on_insert = add_keyed, on_discard = remove_keyed)]
pub struct InstanceKey(Uuid);

impl InstanceKey {
    /// A new key, random (UUIDv4).
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for InstanceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

impl FromStr for InstanceKey {
    type Err = uuid::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(text).map(Self)
    }
}

impl From<Uuid> for InstanceKey {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// The live entity of each key, kept as instances with an [`InstanceKey`] spawn and despawn.
#[derive(Resource, Default, Debug)]
pub struct KeyedInstances(HashMap<InstanceKey, Entity>);

impl KeyedInstances {
    /// The live instance whose key is `key`.
    pub fn get(&self, key: &InstanceKey) -> Option<Entity> {
        self.0.get(key).copied()
    }
}

/// Records the entity under its key. A key another live entity has already is a duplicate, which
/// warns, and the first keeps it.
fn add_keyed(mut world: DeferredWorld, context: HookContext) {
    let key = *world.get::<InstanceKey>(context.entity).unwrap();
    let mut keyed = world.resource_mut::<KeyedInstances>();
    let first = match keyed.0.entry(key) {
        Entry::Occupied(entry) => *entry.get(),
        Entry::Vacant(entry) => {
            entry.insert(context.entity);
            return;
        }
    };
    if first != context.entity {
        warn!(
            "Instance key {key} is on {first} and {}; {first} keeps it",
            context.entity
        );
    }
}

fn remove_keyed(mut world: DeferredWorld, context: HookContext) {
    let key = *world.get::<InstanceKey>(context.entity).unwrap();
    let mut keyed = world.resource_mut::<KeyedInstances>();
    if keyed.get(&key) == Some(context.entity) {
        keyed.0.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key reads back from its hyphenated form.
    #[test]
    fn a_key_reads_back_from_its_text() {
        let key = InstanceKey::generate();
        let text = key.to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(text.parse::<InstanceKey>().unwrap(), key);
    }

    /// The map follows keyed entities as they spawn and despawn, and a duplicate leaves the first.
    #[test]
    fn the_map_follows_spawns_and_despawns() {
        let mut world = World::new();
        world.init_resource::<KeyedInstances>();
        let key = InstanceKey::generate();
        let first = world.spawn(key).id();
        let second = world.spawn(key).id();
        assert_eq!(world.resource::<KeyedInstances>().get(&key), Some(first));
        world.despawn(second);
        assert_eq!(world.resource::<KeyedInstances>().get(&key), Some(first));
        world.despawn(first);
        assert_eq!(world.resource::<KeyedInstances>().get(&key), None);
    }
}
