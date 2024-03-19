use bevy::{
    asset::AssetLoader,
    color::LinearRgba,
    pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

use serde::{Deserialize, Serialize};

use super::{MaterialLoaderError, MaterialParams};

pub type FloorNoisyMaterial = ExtendedMaterial<StandardMaterial, FloorNoisyMaterialExt>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FloorNoisyMaterialParams {
    pub color: LinearRgba,
    pub color_alt: LinearRgba,
    pub roughness: f32,
    pub roughness_alt: f32,
}

impl MaterialParams for FloorNoisyMaterialParams {}

/// AssetLoader for floor materials.
#[derive(Default)]
pub struct FloorNoisyMaterialLoader;

impl AssetLoader for FloorNoisyMaterialLoader {
    type Asset = FloorNoisyMaterial;
    type Settings = ();

    type Error = MaterialLoaderError;

    async fn load<'a>(
        &'a self,
        _reader: &'a mut bevy::asset::io::Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let path = load_context.path().file_stem().unwrap().to_str().unwrap();
        let params = FloorNoisyMaterialParams::decode(path)?;
        let std = StandardMaterial {
            perceptual_roughness: params.roughness,
            ..default()
        };
        let material = FloorNoisyMaterialExt {
            color: params.color,
            color_alt: params.color_alt,
            roughness: params.roughness,
            roughness_alt: params.roughness_alt,
            noise: load_context.load("terrain/textures/noise.png"),
        };
        Ok(ExtendedMaterial {
            base: std,
            extension: material,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["floor-noisy"]
    }
}

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct FloorNoisyMaterialExt {
    #[uniform(100)]
    pub color: LinearRgba,

    #[uniform(101)]
    pub color_alt: LinearRgba,

    #[uniform(102)]
    pub roughness: f32,

    #[uniform(103)]
    pub roughness_alt: f32,

    #[texture(104)]
    #[sampler(105)]
    pub noise: Handle<Image>,
}

impl MaterialExtension for FloorNoisyMaterialExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/floor_noise.wgsl".into()
    }
}
