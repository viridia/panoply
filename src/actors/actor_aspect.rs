use bevy::prelude::*;
use panoply_exemplar::*;

use super::ACTOR_TYPE;

/// 3D models for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct Combatant;

impl Aspect for Combatant {
    fn name(&self) -> &str {
        "Combatant"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Combatant> = RemoveComponent::<Combatant>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
