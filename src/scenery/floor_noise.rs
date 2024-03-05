use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct FloorNoiseMaterial {
    #[uniform(100)]
    pub base: Color,

    #[uniform(101)]
    pub accent: Color,
}

impl MaterialExtension for FloorNoiseMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/floor_noise.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "shaders/floor_noise.wgsl".into()
    }
}
