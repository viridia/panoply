use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
            VertexFormat,
        },
    },
};

pub const ATTRIBUTE_TERRAIN_STYLE: MeshVertexAttribute =
    MeshVertexAttribute::new("terrain_style", 2, VertexFormat::Uint32x2);

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct GroundMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub noise: Handle<Image>,

    #[texture(3)]
    #[sampler(4)]
    pub grass: Handle<Image>,

    #[texture(5)]
    #[sampler(6)]
    pub dirt: Handle<Image>,

    #[texture(7)]
    #[sampler(8)]
    pub moss: Handle<Image>,

    #[texture(9)]
    #[sampler(10)]
    pub cobbles: Handle<Image>,

    #[texture(20, sample_type = "u_int")]
    #[sampler(21)]
    pub biomes: Handle<Image>,

    #[uniform(22)]
    pub water_color: LinearRgba,

    #[uniform(23)]
    pub realm_offset: Vec2,
}

impl Material for GroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
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
            ATTRIBUTE_TERRAIN_STYLE.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
