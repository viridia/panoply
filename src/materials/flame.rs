use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct FlameMaterial {
    #[texture(100)]
    #[sampler(101)]
    pub noise: Handle<Image>,
}

impl Material for FlameMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://panoply/materials/flame.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://panoply/materials/flame.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "embedded://panoply/materials/flame_prepass.wgsl".into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        "embedded://panoply/materials/flame_prepass.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
