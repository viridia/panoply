use bevy::{
    asset::AssetLoader,
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    pbr::ExtendedMaterial,
    prelude::*,
};
use ron::de::from_bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::materials::{FloorNoisyMaterial, FloorNoisyMaterialExt};

#[derive(TypePath, Asset, Serialize, Deserialize, Debug, Default, Clone)]
pub struct FloorSurface {
    pub material: FloorMaterial,
    // nav
    #[serde(default)]
    pub raise: f32,

    #[serde(default = "default_sides")]
    pub sides: bool,

    #[serde(skip)]
    pub material_std: Option<Handle<StandardMaterial>>,

    #[serde(skip)]
    pub material_noisy: Option<Handle<FloorNoisyMaterial>>,
}

fn default_sides() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FloorMaterial {
    Standard {
        #[serde(default)]
        texture: Option<String>,
        #[serde(default, with = "panoply_exemplar::ser::hex_color_opt")]
        color: Option<Srgba>,
        roughness: Option<f32>,
    },
    Unlit {
        #[serde(with = "panoply_exemplar::ser::hex_color")]
        color: Srgba,
    },
    Noise {
        #[serde(with = "panoply_exemplar::ser::hex_color")]
        color: Srgba,
        #[serde(with = "panoply_exemplar::ser::hex_color")]
        color_alt: Srgba,
        roughness: f32,
        roughness_alt: f32,
        // scale: f32,
    },
}

impl Default for FloorMaterial {
    fn default() -> Self {
        FloorMaterial::Standard {
            texture: None,
            color: None,
            roughness: None,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum FloorSurfaceError {
    #[error("Could not load terrain map: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](serde_ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonError(#[from] ron::error::SpannedError),
}

#[derive(Default)]
pub struct FloorSurfaceLoader;

impl AssetLoader for FloorSurfaceLoader {
    type Asset = FloorSurface;
    type Settings = ();
    type Error = FloorSurfaceError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // Deserialize from RON
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut asset = from_bytes::<Self::Asset>(&bytes)?;

        match asset.material {
            FloorMaterial::Standard {
                ref texture,
                color,
                roughness,
            } => {
                let texture: Option<Handle<Image>> = texture.as_ref().map(|t| {
                    load_context
                        .loader()
                        .with_settings(|settings: &mut ImageLoaderSettings| {
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
                        })
                        .load(t)
                });
                let material = StandardMaterial {
                    base_color_texture: texture,
                    base_color: color.unwrap_or(Srgba::WHITE).into(),
                    perceptual_roughness: roughness.unwrap_or(1.0),
                    ..Default::default()
                };
                asset.material_std =
                    Some(load_context.add_labeled_asset("floor_std".to_string(), material));
            }
            FloorMaterial::Unlit { color } => {
                let material = StandardMaterial {
                    base_color: color.into(),
                    unlit: true,
                    ..Default::default()
                };
                asset.material_std =
                    Some(load_context.add_labeled_asset("floor_unlit".to_string(), material));
            }
            FloorMaterial::Noise {
                color,
                color_alt,
                roughness,
                roughness_alt,
            } => {
                let texture: Handle<Image> = load_context.load("terrain/textures/noise.png");
                let material = ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: color.into(),
                        perceptual_roughness: roughness,
                        ..Default::default()
                    },
                    extension: FloorNoisyMaterialExt {
                        color: color.into(),
                        color_alt: color_alt.into(),
                        roughness,
                        roughness_alt,
                        noise: texture,
                    },
                };
                asset.material_noisy =
                    Some(load_context.add_labeled_asset("floor_noisy".to_string(), material));
            }
        }
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["floor.ron"]
    }
}
