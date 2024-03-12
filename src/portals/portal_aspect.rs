use crate::schematic::{Aspect, DetachAspect, InstanceType, ReflectAspect, SimpleDetachAspect};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::TypeId;

#[derive(Debug, Reflect, Clone, Copy, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortalSide {
    #[default]
    Both,
    Front,
    Back,
}

/// 3D models for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct Portal {
    /// Size of the portal aperture. The aperture is a box.
    pub size: Vec3,

    /// Offset of the portal aperture from the center of the entity.
    pub offset: Vec3,

    /// Active side of the portal
    pub side: PortalSide,

    /// When traversing the portal, distance the traversing entity should be displaced.
    /// This prevents the entity from being stuck in the portal.
    pub displacement: Option<f32>,
}

impl Aspect for Portal {
    fn name(&self) -> &str {
        "Portal"
    }

    fn can_apply(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall || meta_type == InstanceType::Fixture
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<Portal> = SimpleDetachAspect::<Portal>::new();
        entity.insert(self.clone());
        &DETACH
    }
}

/// Physics colliders for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct PortalTarget {
    realm: String,
    location: Vec3,
}

impl Aspect for PortalTarget {
    fn name(&self) -> &str {
        "PortalTarget"
    }

    fn can_apply(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall || meta_type == InstanceType::Fixture
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<PortalTarget> = SimpleDetachAspect::<PortalTarget>::new();
        entity.insert(self.clone());
        &DETACH
    }
}
