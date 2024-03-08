use crate::schematic::{Aspect, DetachAspect, InstanceType, ReflectAspect, SimpleDetachAspect};
use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};
use std::any::TypeId;

use super::scenery_colliders::ColliderDesc;

/** Used in archetypes to define the set of models displayed by that entity. */
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ModelComponent {
    /// ID of the model to display.
    pub model: String,

    /// Model rotation in x-axis.
    pub x_rotation: Option<f32>,

    /// Model rotation in y-axis.
    pub y_rotation: Option<f32>,

    /// Model rotation in z-axis.
    pub z_rotation: Option<f32>,

    /// Random variance of models rotation in x-axis.
    pub x_rotation_variance: Option<f32>,

    /// Random variance of models rotation in y-axis.
    pub y_rotation_variance: Option<f32>,

    /// Random variance of models rotation in z-axis.
    pub z_rotation_variance: Option<f32>,

    /// Model translation from tile center.
    pub offset: Option<Vec3>,

    /// Model scale.
    pub scale: Option<f32>,

    /// Random variance of model scale.
    pub scale_variance: Option<f32>,
}

/// 3D models for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct SceneryModels {
    models: Vec<ModelComponent>,
    //     public animations?: IAnimationSpec[];
}

impl Aspect for SceneryModels {
    fn name(&self) -> &str {
        "SceneryModels"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall || meta_type == InstanceType::Fixture
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<SceneryModels> =
            SimpleDetachAspect::<SceneryModels>::new();
        entity.insert(self.clone());
        &DETACH
    }
}

/// Physics colliders for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct SceneryColliders {
    colliders: Vec<ColliderDesc>,
}

impl Aspect for SceneryColliders {
    fn name(&self) -> &str {
        "SceneryColliders"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall || meta_type == InstanceType::Fixture
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<SceneryColliders> =
            SimpleDetachAspect::<SceneryColliders>::new();
        entity.insert(self.clone());
        &DETACH
    }
}

/// Location markers for a given scenery element, used to drive NPC behavior
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct SceneryMarks {
    marks: HashMap<String, Vec<Vec3>>,
}

impl Aspect for SceneryMarks {
    fn name(&self) -> &str {
        "SceneryMarks"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall || meta_type == InstanceType::Fixture
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<SceneryMarks> = SimpleDetachAspect::<SceneryMarks>::new();
        entity.insert(self.clone());
        &DETACH
    }
}
