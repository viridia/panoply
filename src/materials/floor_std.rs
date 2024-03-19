use bevy::{
    asset::AssetLoader,
    color::LinearRgba,
    pbr::StandardMaterial,
    prelude::*,
    render::texture::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
};
use serde::{Deserialize, Serialize};

use super::{MaterialLoaderError, MaterialParams};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FloorStdMaterialParams {
    pub texture: Option<String>,
    pub color: Option<LinearRgba>,
    pub roughness: f32,
    pub unlit: bool,
}

impl MaterialParams for FloorStdMaterialParams {}

/// AssetLoader for floor materials.
#[derive(Default)]
pub struct FloorStdMaterialLoader;

impl AssetLoader for FloorStdMaterialLoader {
    type Asset = StandardMaterial;
    type Settings = ();

    type Error = MaterialLoaderError;

    async fn load<'a>(
        &'a self,
        _reader: &'a mut bevy::asset::io::Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let path = load_context.path().file_stem().unwrap().to_str().unwrap();
        let params = FloorStdMaterialParams::decode(path)?;
        let mut material = StandardMaterial {
            perceptual_roughness: params.roughness,
            unlit: params.unlit,
            ..default()
        };
        if let Some(color) = params.color {
            material.base_color = color.into();
        } else if let Some(texture) = params.texture {
            material.base_color_texture = Some(load_context.load_with_settings(
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
        Ok(material)
    }

    fn extensions(&self) -> &[&str] {
        &["floor-std"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_material_params() {
        let params = FloorStdMaterialParams {
            texture: Some("texture.png".to_string()),
            color: Some(LinearRgba::new(0.5, 0.5, 0.5, 1.0)),
            roughness: 0.5,
            unlit: true,
        };
        let encoded = params.encode().unwrap();
        assert_eq!(
            encoded,
            r#"lKt0ZXh0dXJlLnBuZ5TKPwAAAMo_AAAAyj8AAADKP4AAAMo_AAAAww"#
        );
        let decoded = FloorStdMaterialParams::decode(&encoded).unwrap();
        assert_eq!(params, decoded);
    }
}
