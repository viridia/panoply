use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// Hair: a `StandardMaterial` for its diffuse and a band of shine across the strands, whose
/// direction is the mesh's `TANGENT` (D29). The base's `reflectance` should be 0, or its round
/// highlight shows too.
pub type HairMaterial = ExtendedMaterial<StandardMaterial, HairExtension>;

#[derive(AsBindGroup, TypePath, Debug, Clone, Default, Asset)]
pub struct HairExtension {}

impl MaterialExtension for HairExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://panoply_render/materials/hair.wesl".into()
    }
}
