//! What an evaluation read, for reactions to know what to watch: its tracking scope (see the
//! crate's Terms).

use bevy::ecs::{
    change_detection::Tick, component::ComponentId, entity::Entity, resource::Resource,
    world::World,
};
use bevy::prelude::Component;

/// The components and resources an evaluation read. It only records; whether a change since
/// should run the evaluation again is for its cell to ask, through
/// [`TrackingScope::changed_since`].
#[derive(Debug, Default)]
pub struct TrackingScope {
    /// Each component read: its entity, its id, and whether it was present.
    components: Vec<(Entity, ComponentId, bool)>,
    resources: Vec<ComponentId>,
    /// Every read recorded, repeats included, so that the cost of finding repeats can be judged.
    reads: u32,
}

impl TrackingScope {
    /// Record a read of component `C` on `entity`, which was there or not as `present` says.
    /// A type the world has not registered yet has its id reserved, the id inserting it will use,
    /// so that the read notices it arriving.
    pub fn track_component<C: Component>(&mut self, entity: Entity, world: &World, present: bool) {
        let id = world.components_queue().queue_register_component::<C>();
        self.track_component_id(entity, id, present);
    }

    pub fn track_component_id(&mut self, entity: Entity, component: ComponentId, present: bool) {
        self.reads += 1;
        let read = (entity, component, present);
        if !self.components.contains(&read) {
            self.components.push(read);
        }
    }

    /// Record a read of resource `R`.
    pub fn track_resource<R: Resource>(&mut self, world: &World) {
        let Some(id) = world.component_id::<R>() else {
            return;
        };
        self.reads += 1;
        if !self.resources.contains(&id) {
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

    /// Every read recorded, repeats included.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// The reads that repeated one already recorded.
    pub fn repeats(&self) -> u32 {
        self.reads - (self.components.len() + self.resources.len()) as u32
    }

    /// Forget every read, for the evaluation to run again.
    pub fn clear(&mut self) {
        self.components.clear();
        self.resources.clear();
        self.reads = 0;
    }

    /// Whether anything read has changed since `last_run`: a component or resource written, a
    /// component added to or removed from its entity, or the entity despawned.
    pub fn changed_since(&self, world: &World, last_run: Tick, this_run: Tick) -> bool {
        let component_changed = |&(entity, id, present): &(Entity, ComponentId, bool)| {
            let ticks = world
                .get_entity(entity)
                .ok()
                .and_then(|entity| entity.get_change_ticks_by_id(id));
            match ticks {
                Some(ticks) => !present || ticks.is_changed(last_run, this_run),
                None => present,
            }
        };
        let resource_changed = |&id: &ComponentId| {
            world
                .get_resource_change_ticks_by_id(id)
                .is_none_or(|ticks| ticks.is_changed(last_run, this_run))
        };
        self.components.iter().any(component_changed) || self.resources.iter().any(resource_changed)
    }
}
