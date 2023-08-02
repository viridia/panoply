use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
            VertexFormat,
        },
    },
};

pub const ATTRIBUTE_DEPTH_MOTION: MeshVertexAttribute =
    MeshVertexAttribute::new("depth_motion", 0x1000, VertexFormat::Float32x3);

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "e9fd10a4-0a9d-409a-ae07-0142783fc98d"]
pub struct WaterMaterial {
    #[uniform(1)]
    pub water_color: Color,

    #[texture(2)]
    #[sampler(3)]
    pub noise: Handle<Image>,

    #[texture(4)]
    #[sampler(5)]
    pub sky: Handle<Image>,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "terrain/shaders/water.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "terrain/shaders/water.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            ATTRIBUTE_DEPTH_MOTION.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct WaterMaterialResource {
    pub handle: Handle<WaterMaterial>,
}

pub fn create_water_material(
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut resource: ResMut<WaterMaterialResource>,
    asset_server: Res<AssetServer>,
) {
    resource.handle = materials.add(WaterMaterial {
        water_color: Color::rgb(0.0, 0.3, 0.0),
        noise: asset_server.load("terrain/textures/water-waves-2.png"),
        sky: asset_server.load("terrain/textures/water-sky.png"),
    });
}
