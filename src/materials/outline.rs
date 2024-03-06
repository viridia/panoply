use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct OutlineMaterial {
    #[uniform(100)]
    pub width: f32,
}

impl MaterialExtension for OutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }
}
