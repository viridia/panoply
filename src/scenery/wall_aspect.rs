use std::any::TypeId;

use crate::schematic::{Aspect, DetachAspect, InstanceType, ReflectAspect, SimpleDetachAspect};
use bevy::prelude::*;

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

    fn can_apply(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<WallSize> = SimpleDetachAspect::<WallSize>::new();
        entity.insert(self.clone());
        &DETACH
    }
}
