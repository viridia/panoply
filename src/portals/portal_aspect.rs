use crate::{
    msgpack::Vector3,
    scenery::{FIXTURE_TYPE, WALL_TYPE},
};
use bevy::prelude::*;
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};
use std::any::TypeId;

#[derive(Debug, Reflect, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortalSide {
    #[default]
    Both,
    Front,
    Back,
}

/// Defines geometry for a portal.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct Portal {
    /// Half-size of the portal aperture.
    pub size: Vec2,

    /// Portal rotation.
    pub x_rotation: Option<f32>,
    pub y_rotation: Option<f32>,
    pub z_rotation: Option<f32>,

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

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Portal> = RemoveComponent::<Portal>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Defines the remote location of a portal.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct PortalTarget {
    pub(crate) realm: String,
    pub(crate) pos: Vector3,
}

impl Aspect for PortalTarget {
    fn name(&self) -> &str {
        "PortalTarget"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<PortalTarget> = RemoveComponent::<PortalTarget>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
