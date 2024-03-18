use crate::{reflect_types::HexColor, scenery::floor_noise::FloorNoiseMaterial};
use bevy::{
    pbr::ExtendedMaterial,
    prelude::*,
    render::texture::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
};
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
    // water_current_x: Option<f32>,
    // water_current_y: Option<f32>,
}

impl Aspect for StdFloorSurface {
    fn name(&self) -> &str {
        "StdFloorSurface"
    }

    fn can_attach(&self, meta_type: panoply_exemplar::InstanceType) -> bool {
        meta_type == FLOOR_TYPE
    }

    fn load_dependencies(&mut self, _label: &str, load_context: &mut bevy::asset::LoadContext) {
        let mut material_name = String::with_capacity(64);
        material_name.push_str("Floor.Material");
        if let Some(texture) = &self.texture {
            material_name.push('.');
            material_name.push_str(texture);
        } else if let Some(color) = &self.color {
            material_name.push('.');
            material_name.push_str(color.0.to_hex().as_str());
        }
        if let Some(roughness) = self.roughness {
            material_name.push('.');
            material_name.push_str(&roughness.to_string());
        }
        if self.unlit.unwrap_or(false) {
            material_name.push_str(".Unlit");
        }
        // println!("Loading material: {}", material_name);

        self.material = load_context.labeled_asset_scope(material_name, |lc| {
            let mut material = StandardMaterial {
                perceptual_roughness: self.roughness.unwrap_or(1.0),
                unlit: self.unlit.unwrap_or(false),
                ..default()
            };
            if let Some(color) = &self.color {
                material.base_color = color.0.into();
            } else if let Some(texture) = &self.texture {
                material.base_color_texture = Some(lc.load_with_settings(
                    texture,
                    |settings: &mut ImageLoaderSettings| {
                        settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                            label: Some("Floor Region".to_string()),
                            address_mode_u: ImageAddressMode::Repeat,
                            address_mode_v: ImageAddressMode::Repeat,
                            address_mode_w: ImageAddressMode::Repeat,
                            mag_filter: ImageFilterMode::Linear,
                            min_filter: ImageFilterMode::Linear,
                            mipmap_filter: ImageFilterMode::Linear,
                            ..default()
                        });
                    },
                ));
            }
            // material.base_color_texture_transform = TextureTransform {
            //     offset: Vec2::new(0.0, 0.0),
            //     scale: Vec2::new(1.0, 1.0),
            //     rotation: 0.0,
            // };
            material
        });
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
    pub(crate) material: Handle<ExtendedMaterial<StandardMaterial, FloorNoiseMaterial>>,
}

impl Aspect for NoiseFloorSurface {
    fn name(&self) -> &str {
        "NoiseFloorSurface"
    }

    fn can_attach(&self, meta_type: panoply_exemplar::InstanceType) -> bool {
        meta_type == FLOOR_TYPE
    }

    fn load_dependencies(&mut self, label: &str, load_context: &mut bevy::asset::LoadContext) {
        // println!("Loading material: {}.NoiseFloorSurface.Material", label);
        self.material = load_context.labeled_asset_scope(
            format!("{}.NoiseFloorSurface.Material", label),
            |lc| {
                // println!("Loading material: {}.NoiseFloorSurface.Material", label);
                let std = StandardMaterial {
                    perceptual_roughness: self.roughness.unwrap_or(1.0),
                    ..default()
                };
                let material = FloorNoiseMaterial {
                    color: self.color.into(),
                    color_alt: self.color_alt.into(),
                    roughness: self.roughness.unwrap_or(1.0),
                    roughness_alt: self.roughness_alt.unwrap_or(self.roughness.unwrap_or(1.0)),
                    noise: lc.load("terrain/textures/noise.png"),
                };
                ExtendedMaterial {
                    base: std,
                    extension: material,
                }
            },
        );
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
