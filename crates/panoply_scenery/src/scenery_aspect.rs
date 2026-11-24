use std::collections::BTreeMap;

use bevy::{
    annotations::{DisplayName, Step},
    prelude::*,
};
use panoply_core::HexColor;
use panoply_exemplar::{
    annotations::{AspectIcon, PropertySummary, ReflectPropertySummary},
    *,
};
use serde::{Deserialize, Serialize};

use super::scenery_colliders::ColliderDesc;

/// A scenery model: a `file#Scene` path to a named scene of a glTF file in `scenery/models` or
/// `terrain/models`.
#[derive(Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct SceneryModel(pub String);

/** Used in archetypes to define the set of models displayed by that entity. */
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize, PropertySummary)]
pub struct ModelComponent {
    /// The model to display.
    #[reflect(@DisplayName("Model"))]
    pub asset: SceneryModel,

    /// Model rotation in x-axis.
    #[reflect(@DisplayName("X rotation"))]
    pub x_rotation: Option<f32>,

    /// Model rotation in y-axis.
    #[reflect(@DisplayName("Y rotation"))]
    pub y_rotation: Option<f32>,

    /// Model rotation in z-axis.
    #[reflect(@DisplayName("Z rotation"))]
    pub z_rotation: Option<f32>,

    /// Random variance of models rotation in x-axis.
    #[reflect(@DisplayName("X rotation variance"))]
    pub x_rotation_variance: Option<f32>,

    /// Random variance of models rotation in y-axis.
    #[reflect(@DisplayName("Y rotation variance"))]
    pub y_rotation_variance: Option<f32>,

    /// Random variance of models rotation in z-axis.
    #[reflect(@DisplayName("Z rotation variance"))]
    pub z_rotation_variance: Option<f32>,

    /// Model translation from tile center.
    #[reflect(@Step(0.05), @DisplayName("Offset"))]
    pub offset: Option<Vec3>,

    /// Model scale.
    #[reflect(@DisplayName("Scale"))]
    pub scale: Option<f32>,

    /// Random variance of model scale.
    #[reflect(@DisplayName("Scale variance"))]
    pub scale_variance: Option<f32>,
}

impl PropertySummary for ModelComponent {
    /// The model's scene, as "MossyBrickWallHalf", without its file.
    fn property_summary(&self) -> String {
        let path = &self.asset.0;
        path.rsplit_once('#')
            .map_or(path.as_str(), |(_, scene)| scene)
            .to_string()
    }
}

/// 3D models for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/box.png"))]
pub struct SceneryModels(pub Vec<ModelComponent>);
//     public animations?: IAnimationSpec[];

impl NativeAspect for SceneryModels {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Wall || kind == InstanceKind::Fixture
    }
}

/// Physics colliders for a given scenery element.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/square-dashed.png"))]
pub struct SceneryColliders(pub Vec<ColliderDesc>);

impl NativeAspect for SceneryColliders {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Wall || kind == InstanceKind::Fixture
    }
}

/// Location markers for a given scenery element, used to drive NPC behavior
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/map-pin.png"))]
pub struct SceneryMarks(pub BTreeMap<String, Vec<Vec3>>);

impl NativeAspect for SceneryMarks {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Wall || kind == InstanceKind::Fixture
    }
}

/// Location markers for a given scenery element, used to drive NPC behavior
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
#[reflect(@AspectIcon("editor/icons/lucide/lightbulb.png"))]
pub struct LightSource {
    /// Position of the effect relative to the fixture or wall.
    #[reflect(@Step(0.05), @DisplayName("Offset"))]
    offset: Option<Vec3>,

    /// Radius of point light (max distance).
    #[reflect(@Step(0.05), @DisplayName("Radius"))]
    radius: Option<f32>,

    /// Color of the light.
    #[reflect(@DisplayName("Color"))]
    color: Option<HexColor>,

    /// Intensity of the light.
    #[reflect(@DisplayName("Intensity"))]
    intensity: Option<f32>,

    /// If present, effect is only enabled when this instance property is true.
    #[reflect(@DisplayName("Enabled"))]
    enabled: Option<bool>,
}

impl NativeAspect for LightSource {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Wall || kind == InstanceKind::Fixture
    }
}
