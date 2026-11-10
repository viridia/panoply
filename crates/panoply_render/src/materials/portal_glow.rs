use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

/// The shimmer around a portal: noise over the mesh, drifting off its surface, in the mesh's
/// vertex colors. Additive, so it draws in no prepass; meshes using it must not cast shadows.
#[derive(AsBindGroup, TypePath, Debug, Clone, Default, Asset)]
pub struct PortalGlowMaterial {}

impl Material for PortalGlowMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://panoply_render/materials/portal_glow.wesl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://panoply_render/materials/portal_glow.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
