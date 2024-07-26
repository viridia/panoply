use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct BlackMaterialExtension {}

impl MaterialExtension for BlackMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }
}

pub type BlackMaterial = ExtendedMaterial<StandardMaterial, BlackMaterialExtension>;
