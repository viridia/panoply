use crate::{
    materials::{
        FloorNoisyMaterial, FloorNoisyMaterialParams, FloorStdMaterialParams, MaterialParams,
    },
    reflect_types::HexColor,
};
use bevy::prelude::*;
use panoply_exemplar::*;

use super::FLOOR_TYPE;

/// Floor surface aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct StdFloorSurface {
    /// Texture to use for this floor
    texture: Option<String>,

    /// Surface color if no texture is used.
    color: Option<HexColor>,

    /// Surface roughness
    roughness: Option<f32>,

    /// Whether the surface should be unlit
    unlit: Option<bool>,

    /// Cached material handle
    #[reflect(ignore)]
    pub(crate) material: Handle<StandardMaterial>,
}

impl Aspect for StdFloorSurface {
    fn name(&self) -> &str {
        "StdFloorSurface"
    }

    fn can_attach(&self, meta_type: panoply_exemplar::InstanceType) -> bool {
        meta_type == FLOOR_TYPE
    }

    fn load_dependencies(&mut self, _label: &str, load_context: &mut bevy::asset::LoadContext) {
        let params = FloorStdMaterialParams {
            texture: self.texture.clone(),
            color: self.color.map(|c| c.0.into()),
            roughness: self.roughness.unwrap_or(1.0),
            unlit: self.unlit.unwrap_or(false),
        };
        self.material =
            load_context.load(format!("inline://{}.floor-std", params.encode().unwrap()));
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<StdFloorSurface> = RemoveComponent::<StdFloorSurface>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Floor surface aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct NoiseFloorSurface {
    /// Noise base color
    color: HexColor,

    /// Accent color
    color_alt: HexColor,

    /// Surface roughness
    roughness: Option<f32>,

    /// Surface roughness
    roughness_alt: Option<f32>,

    #[reflect(ignore)]
    pub(crate) material: Handle<FloorNoisyMaterial>,
}

impl Aspect for NoiseFloorSurface {
    fn name(&self) -> &str {
        "NoiseFloorSurface"
    }

    fn can_attach(&self, meta_type: panoply_exemplar::InstanceType) -> bool {
        meta_type == FLOOR_TYPE
    }

    fn load_dependencies(&mut self, _label: &str, load_context: &mut bevy::asset::LoadContext) {
        let roughness = self.roughness.unwrap_or(1.0);
        let params = FloorNoisyMaterialParams {
            color: self.color.into(),
            color_alt: self.color_alt.into(),
            roughness,
            roughness_alt: self.roughness_alt.unwrap_or(roughness),
        };
        self.material =
            load_context.load(format!("inline://{}.floor-noisy", params.encode().unwrap()));
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<NoiseFloorSurface> =
            RemoveComponent::<NoiseFloorSurface>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Floor geometry aspect
#[derive(Component, Debug, Reflect, Clone, Copy, Default)]
#[reflect(Aspect, Default)]
pub struct FloorGeometry {
    /// How far up the floor should be raised or lowered.
    pub(crate) raise: Option<f32>,

    /// Whether to render the sides of this floor.
    pub(crate) sides: Option<bool>,
}

impl Aspect for FloorGeometry {
    fn name(&self) -> &str {
        "FloorGeometry"
    }

    fn can_attach(&self, meta_type: panoply_exemplar::InstanceType) -> bool {
        meta_type == FLOOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<FloorGeometry> = RemoveComponent::<FloorGeometry>::new();

        entity.insert(*self);
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(*self)
    }
}

/// Floor navigation aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct FloorNav {
    blocked: bool,
}

impl Aspect for FloorNav {
    fn name(&self) -> &str {
        "FloorNav"
    }

    fn can_attach(&self, meta_type: panoply_exemplar::InstanceType) -> bool {
        meta_type == FLOOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<FloorNav> = RemoveComponent::<FloorNav>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
