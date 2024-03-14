use std::{any::TypeId, sync::Arc};

use super::{Aspect, InstanceAspects, Schematic, SchematicData};
use crate::schematic::aspect;
use aspect::{DetachAspect, OwnedAspects};
use bevy::{
    ecs::system::EntityCommand,
    prelude::*,
    utils::{hashbrown::HashMap, smallvec::SmallVec},
};

/// Custom command that updates an entity's components guided by a schematic.
pub struct UpdateAspects<B: Bundle> {
    /// Schematic to attach to entity
    pub(crate) schematic: Handle<Schematic>,

    /// Components to insert after applying schematic, used to trigger post-processing.
    pub(crate) finish: B,
}

impl<B: Bundle> EntityCommand for UpdateAspects<B> {
    fn apply(self, id: Entity, world: &mut World) {
        let schematic_assets = world.get_resource::<Assets<Schematic>>().unwrap();
        let mut schematics: SmallVec<[Arc<SchematicData>; 8]> = SmallVec::new();
        let mut shandle = &self.schematic;
        loop {
            let Some(schematic) = schematic_assets.get(shandle) else {
                break;
            };
            schematics.push(schematic.0.clone());
            if let Some(ref next) = schematic.0.extends {
                shandle = &next;
            } else {
                break;
            }
        }

        if let Some(mut entity) = world.get_entity_mut(id) {
            // Get the set of aspects currently owned.
            let mut to_remove: HashMap<TypeId, &'static dyn DetachAspect> =
                match entity.get_mut::<OwnedAspects>() {
                    Some(mut owned_aspects) => std::mem::take(&mut owned_aspects.0),
                    None => HashMap::with_capacity(0),
                };

            // Keep track of aspects as we add them.
            let mut next_owned: HashMap<TypeId, &'static dyn DetachAspect> =
                HashMap::with_capacity(0);

            // First process aspects on the instance
            let mut aspects_copy: Vec<Box<dyn Aspect>> = Vec::new();
            if let Some(mut instance_aspects) = entity.get_mut::<InstanceAspects>() {
                std::mem::swap(&mut instance_aspects.0, &mut aspects_copy);
            }

            for aspect in aspects_copy.iter() {
                let aspect_type = aspect.id();
                if !next_owned.contains_key(&aspect_type) {
                    next_owned.insert(aspect_type, aspect.apply(&mut entity));
                    to_remove.remove(&aspect_type);
                }
            }

            if let Some(mut instance_aspects) = entity.get_mut::<InstanceAspects>() {
                std::mem::swap(&mut instance_aspects.0, &mut aspects_copy);
            }

            // Loop through inheritance chain
            for schematic in schematics.iter() {
                for aspect in schematic.aspects.iter() {
                    // Only add aspect if no other aspect of the same type has been added.
                    let aspect_type = aspect.id();
                    if !next_owned.contains_key(&aspect_type) {
                        next_owned.insert(aspect_type, aspect.apply(&mut entity));
                        to_remove.remove(&aspect_type);
                    }
                }
            }

            for (_, remover) in to_remove.iter() {
                remover.detach_aspect(&mut entity);
            }

            entity.insert((OwnedAspects(next_owned), self.finish));
        }
    }
}
