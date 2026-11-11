//! What an evaluation read, for reactions to know what to watch.

use bevy::ecs::{component::ComponentId, entity::Entity, resource::Resource, world::World};
use bevy::prelude::Component;

/// The components and resources an evaluation read. It only records: deciding which changes
/// run the evaluation again is the reaction passes' business.
#[derive(Debug, Default)]
pub struct TrackingScope {
    /// Each component read: its entity, its id, and whether it was present.
    components: Vec<(Entity, ComponentId, bool)>,
    resources: Vec<ComponentId>,
}

impl TrackingScope {
    /// Record a read of component `C` on `entity`, which was there or not as `present` says.
    /// A component the world has never registered cannot have been present, so is not recorded.
    pub fn track_component<C: Component>(&mut self, entity: Entity, world: &World, present: bool) {
        if let Some(id) = world.component_id::<C>() {
            self.track_component_id(entity, id, present);
        }
    }

    pub fn track_component_id(&mut self, entity: Entity, component: ComponentId, present: bool) {
        let read = (entity, component, present);
        if !self.components.contains(&read) {
            self.components.push(read);
        }
    }

    /// Record a read of resource `R`.
    pub fn track_resource<R: Resource>(&mut self, world: &World) {
        if let Some(id) = world.component_id::<R>()
            && !self.resources.contains(&id)
        {
            self.resources.push(id);
        }
    }

    /// The components read, as (entity, component, whether it was present).
    pub fn components(&self) -> &[(Entity, ComponentId, bool)] {
        &self.components
    }

    /// The resources read.
    pub fn resources(&self) -> &[ComponentId] {
        &self.resources
    }
}
