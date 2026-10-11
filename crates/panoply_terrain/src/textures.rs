use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};

/// Sampler for tiling textures: repeats in every direction and filters between mip levels.
/// The terrain textures are KTX2 files with full mip chains, made by `pipeline.ts`.
pub fn tiling_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: Some("Tiling texture".to_string()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    })
}

/// Load a tiling texture with [`tiling_sampler`].
pub fn load_tiling_texture(server: &AssetServer, path: &'static str) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| settings.sampler = tiling_sampler())
        .load(path)
}
