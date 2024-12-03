use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType,
            SpecializedMeshPipelineError, VertexFormat,
        },
    },
};

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct BiomeSurfaceAttrs {
    /// Surface roughness.
    roughness: f32,
    blend_var: f32,
    blend_t0: f32,
    blend_t1: f32,
    edge_var: f32,
    edge_t0: f32,
    edge_t1: f32,

    // Texture scale
    tx_scale: f32,

    // Darkened color which shows up at edges of top surface (near roads etc.).
    edge_tint: LinearRgba,
}

impl BiomeSurfaceAttrs {
    pub const GRASS: BiomeSurfaceAttrs = BiomeSurfaceAttrs {
        roughness: 0.99,
        blend_var: 0.1,
        blend_t0: 0.4,
        blend_t1: 0.5,
        edge_var: 0.5,
        edge_t0: 0.45,
        edge_t1: 0.9,
        tx_scale: 0.1,
        edge_tint: LinearRgba::new(0.65, 0.65, 0.65, 1.0),
    };
}

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct GroundMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub noise: Handle<Image>,

    #[texture(3)]
    #[sampler(4)]
    pub dirt: Handle<Image>,

    #[texture(5)]
    #[sampler(6)]
    pub cobbles: Handle<Image>,

    #[texture(7)]
    #[sampler(8)]
    pub surface0: Handle<Image>,

    #[texture(9)]
    #[sampler(10)]
    pub surface1: Handle<Image>,

    #[uniform(20)]
    pub water_color: LinearRgba,

    #[uniform(21)]
    pub biome_weight: Mat4,

    #[uniform(22)]
    pub biome_surface: [BiomeSurfaceAttrs; 4],
}

impl GroundMaterial {
    pub const ATTRIBUTE_TERRAIN_STYLE: MeshVertexAttribute =
        MeshVertexAttribute::new("terrain_style", 200, VertexFormat::Uint32x2);
}

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
            GroundMaterial::ATTRIBUTE_TERRAIN_STYLE.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

#[derive(Resource)]
pub struct GroundMaterialCache {
    /// The noise texture used to generate the terrain.
    pub noise: Handle<Image>,

    // Road and path textures. Also default texture for dirt biomes.
    pub dirt: Handle<Image>,
    pub cobbles: Handle<Image>,

    // Biome textures.
    pub grass: Handle<Image>,
    pub moss: Handle<Image>,
    // pub sand: Handle<Image>,
    // pub snow: Handle<Image>,
}

impl FromWorld for GroundMaterialCache {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource_mut::<AssetServer>();
        GroundMaterialCache {
            noise: assets.load("terrain/textures/noise.png"),
            dirt: assets.load("terrain/textures/dirt.png"),
            cobbles: assets.load("terrain/textures/cobbles.png"),
            grass: assets.load("terrain/textures/grass.png"),
            moss: assets.load("terrain/textures/moss.png"),
            // sand: assets.load("textures/sand.png"),
            // snow: assets.load("textures/snow.png"),
        }
    }
}

impl GroundMaterialCache {
    pub fn get_material(&self, materials: &mut Assets<GroundMaterial>) -> Handle<GroundMaterial> {
        materials.add(GroundMaterial {
            noise: self.noise.clone(),
            dirt: self.dirt.clone(),
            cobbles: self.cobbles.clone(),
            surface0: self.grass.clone(),
            surface1: self.moss.clone(),
            water_color: Srgba::rgb(0.0, 0.1, 0.3).into(),
            biome_weight: Mat4::from_cols(
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec4::new(0.0, 1.0, 1.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 0.0, 0.0),
            ),
            biome_surface: [
                BiomeSurfaceAttrs::GRASS,
                BiomeSurfaceAttrs::GRASS,
                BiomeSurfaceAttrs::GRASS,
                BiomeSurfaceAttrs::GRASS,
            ],
        })
    }
}
