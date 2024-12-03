use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
        },
    },
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct GroundMaterial {}

impl Material for GroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://panoply_terrain/shaders/ground.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "embedded://panoply_terrain/shaders/ground.wgsl".into()
    }

    // fn prepass_vertex_shader() -> ShaderRef {
    //     "terrain/shaders/ground.wgsl".into()
    // }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            // ATTRIBUTE_TERRAIN_STYLE.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
