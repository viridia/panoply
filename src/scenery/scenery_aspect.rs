use crate::reflect_types::HexColor;
use bevy::{prelude::*, utils::HashMap};
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};

use super::{scenery_colliders::ColliderDesc, FIXTURE_TYPE, WALL_TYPE};

/** Used in archetypes to define the set of models displayed by that entity. */
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ModelComponent {
    /// ID of the model to display.
    pub asset: String,

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
    pub models: Vec<ModelComponent>,
    //     public animations?: IAnimationSpec[];
}

impl Aspect for SceneryModels {
    fn name(&self) -> &str {
        "SceneryModels"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<SceneryModels> = RemoveComponent::<SceneryModels>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
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
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<SceneryColliders> =
            RemoveComponent::<SceneryColliders>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
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
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<SceneryMarks> = RemoveComponent::<SceneryMarks>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Location markers for a given scenery element, used to drive NPC behavior
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct LightSource {
    /// Position of the effect relative to the fixture or wall.
    offset: Option<Vec3>,

    /// Radius of point light (max distance).
    radius: Option<f32>,

    /// Color of the light.
    color: Option<HexColor>,

    /// Intensity of the light.
    intensity: Option<f32>,

    /// If present, effect is only enabled when this instance property is true.
    enabled: Option<bool>,
}

impl Aspect for LightSource {
    fn name(&self) -> &str {
        "LightSource"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<LightSource> = RemoveComponent::<LightSource>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
