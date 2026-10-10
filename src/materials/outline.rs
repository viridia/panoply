use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct OutlineMaterialExtension {
    #[uniform(100)]
    pub width: f32,
}

impl MaterialExtension for OutlineMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        "embedded://panoply/materials/outline.wesl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://panoply/materials/outline.wesl".into()
    }
}

pub type OutlineMaterial = ExtendedMaterial<StandardMaterial, OutlineMaterialExtension>;
