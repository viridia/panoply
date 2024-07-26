use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct OutlineMaterialExtension {
    #[uniform(100)]
    pub width: f32,
}

impl MaterialExtension for OutlineMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }
}

pub type OutlineMaterial = ExtendedMaterial<StandardMaterial, OutlineMaterialExtension>;
