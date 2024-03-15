use std::any::TypeId;

use bevy::prelude::*;
use panoply_exemplar::*;

use super::WALL_TYPE;

/// Wall Size aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct WallSize {
    /// How many tile spaces does this wall take up in the x direction
    x: u32,

    /// How many tile spaces does this wall take up in the y (actually z) direction
    y: u32,
}

impl Aspect for WallSize {
    fn name(&self) -> &str {
        "WallSize"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<WallSize> = RemoveComponent::<WallSize>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
