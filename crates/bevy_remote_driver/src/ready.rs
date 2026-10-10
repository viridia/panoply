//! Scene readiness as state a query can see (DD3.2).

use bevy_ecs::{component::Component, observer::On, reflect::ReflectComponent, system::Commands};
use bevy_reflect::{Reflect, std_traits::ReflectDefault};
use bevy_scene::Ready;

/// Marks an entity whose scene has finished spawning, along with the assets that scene names.
///
/// Bevy announces this with a [`Ready`] event, which a test client connecting afterwards would
/// miss. This component stays, so a client can wait for a scene that became ready before it started
/// looking. Assets requested after the scene spawned, such as an image a system loads later, are
/// not covered.
#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
#[reflect(Component, Default)]
pub struct SceneReady;

pub(crate) fn mark_ready(ready: On<Ready>, mut commands: Commands) {
    // `try_insert`: a scene entity may be despawned before the command is applied.
    commands.entity(ready.entity).try_insert(SceneReady);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    #[test]
    fn a_ready_entity_keeps_its_marker() {
        let mut world = World::new();
        world.add_observer(mark_ready);
        let root = world.spawn_empty().id();
        let other = world.spawn_empty().id();

        world.trigger(Ready { entity: root });
        world.flush();

        assert!(world.get::<SceneReady>(root).is_some());
        assert!(world.get::<SceneReady>(other).is_none());
    }
}
