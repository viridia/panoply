use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct FloorNoiseMaterial {
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

impl MaterialExtension for FloorNoiseMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/floor_noise.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "shaders/floor_noise.wgsl".into()
    }
}
