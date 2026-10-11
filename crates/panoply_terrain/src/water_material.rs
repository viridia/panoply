use bevy::{
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexFormat,
    },
    shader::ShaderRef,
};

pub const ATTRIBUTE_DEPTH_MOTION: MeshVertexAttribute =
    MeshVertexAttribute::new("depth_motion", 0x1000, VertexFormat::Float32x3);

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct WaterMaterial {
    #[uniform(1)]
    pub water_color: LinearRgba,

    #[uniform(2)]
    pub sky_color: [LinearRgba; 2],

    #[texture(3)]
    #[sampler(4)]
    pub waves: Handle<Image>,

    #[texture(5)]
    #[sampler(6)]
    pub sky: Handle<Image>,

    #[texture(7)]
    #[sampler(8)]
    pub foam: Handle<Image>,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://panoply_terrain/shaders/water.wesl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "embedded://panoply_terrain/shaders/water.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
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

/// The color the three.js `LakeMaterial` showed for `color`. It converted its uniforms to
/// linear but never encoded its output (no `encodings_fragment`), so the linear values reached
/// the sRGB canvas unconverted: the shown color is the linear values read as sRGB.
fn as_displayed_by_threejs(color: Srgba) -> LinearRgba {
    let linear = LinearRgba::from(color);
    Srgba::rgb(linear.red, linear.green, linear.blue).into()
}

pub fn create_water_material(
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut resource: ResMut<WaterMaterialResource>,
    asset_server: Res<AssetServer>,
) {
    resource.handle = materials.add(WaterMaterial {
        water_color: as_displayed_by_threejs(Srgba::rgb(0.0, 0.3, 0.3)),
        sky_color: [
            as_displayed_by_threejs(Srgba::rgb(0.7, 0.8, 0.9)),
            as_displayed_by_threejs(Srgba::rgb(0.6, 0.7, 0.9)),
        ],
        waves: asset_server.load("terrain/textures/water-waves-normal.png"),
        sky: asset_server.load("terrain/textures/water-clouds.png"),
        foam: asset_server.load("terrain/textures/noise.png"),
    });
}
