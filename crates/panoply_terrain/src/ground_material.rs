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

use crate::{BiomeSurfaceType, BiomesTable, ParcelBiomes};

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
        tx_scale: 0.2,
        edge_tint: LinearRgba::new(0.65, 0.65, 0.65, 1.0),
    };

    pub const MOSS: BiomeSurfaceAttrs = BiomeSurfaceAttrs {
        roughness: 0.99,
        blend_var: 0.1,
        blend_t0: 0.4,
        blend_t1: 0.5,
        edge_var: 0.5,
        edge_t0: 0.45,
        edge_t1: 0.9,
        tx_scale: 0.2,
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
    pub texture0: Handle<Image>,

    #[texture(9)]
    #[sampler(10)]
    pub texture1: Handle<Image>,

    #[texture(11)]
    #[sampler(12)]
    pub texture2: Handle<Image>,

    #[texture(13)]
    #[sampler(14)]
    pub texture3: Handle<Image>,

    #[uniform(20)]
    pub water_color: LinearRgba,

    #[uniform(21)]
    pub biome_weight: Mat4,

    #[uniform(22)]
    pub biome_surface: [BiomeSurfaceAttrs; 8],
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
    pub sand: Handle<Image>,
    pub snow: Handle<Image>,
    pub taiga: Handle<Image>,

    pub material: Option<Handle<GroundMaterial>>,
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
            sand: assets.load("terrain/textures/sand.png"),
            snow: assets.load("terrain/textures/snow.png"),
            taiga: assets.load("terrain/textures/taiga2.png"),
            material: None,
            // sand: assets.load("textures/sand.png"),
            // snow: assets.load("textures/snow.png"),
        }
    }
}

impl GroundMaterialCache {
    pub fn get_material(
        &mut self,
        materials: &mut Assets<GroundMaterial>,
        parcel_biomes: ParcelBiomes,
        biomes_table: &BiomesTable,
    ) -> Handle<GroundMaterial> {
        let biomes: [BiomeSurfaceType; 4] = [
            biomes_table.get_biome(parcel_biomes[0]).surface,
            biomes_table.get_biome(parcel_biomes[1]).surface,
            biomes_table.get_biome(parcel_biomes[2]).surface,
            biomes_table.get_biome(parcel_biomes[3]).surface,
        ];
        println!("parcel_biomes: {:?}", biomes);
        let mut biome_textures: [Handle<Image>; 4] = [
            Handle::default(),
            Handle::default(),
            Handle::default(),
            Handle::default(),
        ];
        let mut biome_weight = Mat4::ZERO;
        // biome_weight
        //     .col_mut(0)
        //     .clone_from(&Vec4::new(1.0, 1.0, 1.0, 1.0));
        // biome_weight.y_axis = Vec4::new(0.0, 1.0, 1.0, 1.0);
        let mut biome_surface = [BiomeSurfaceAttrs::GRASS; 8];
        let mut surface_index = 0usize;

        // if biomes.contains(&crate::BiomeSurfaceType::Dirt) {}
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Grass) {
            biome_textures[surface_index] = self.grass.clone();
            biome_surface[surface_index] = BiomeSurfaceAttrs::GRASS;
            biome_weight
                .col_mut(surface_index)
                .clone_from(&get_biome_weights(&biomes, crate::BiomeSurfaceType::Grass));
            surface_index += 1;
        }
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Moss) {
            biome_textures[surface_index] = self.moss.clone();
            biome_surface[surface_index] = BiomeSurfaceAttrs::MOSS;
            biome_weight
                .col_mut(surface_index)
                .clone_from(&get_biome_weights(&biomes, crate::BiomeSurfaceType::Moss));
            surface_index += 1;
        }
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Sand) {
            biome_textures[surface_index] = self.sand.clone();
            biome_surface[surface_index] = BiomeSurfaceAttrs::MOSS;
            biome_weight
                .col_mut(surface_index)
                .clone_from(&get_biome_weights(&biomes, crate::BiomeSurfaceType::Sand));
            surface_index += 1;
        }
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Snow) {
            biome_textures[surface_index] = self.snow.clone();
            biome_surface[surface_index] = BiomeSurfaceAttrs::MOSS;
            biome_weight
                .col_mut(surface_index)
                .clone_from(&get_biome_weights(&biomes, crate::BiomeSurfaceType::Snow));
            surface_index += 1;
        }
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Tundra) {}
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Taiga) {
            biome_textures[surface_index] = self.taiga.clone();
            biome_surface[surface_index] = BiomeSurfaceAttrs::MOSS;
            biome_weight
                .col_mut(surface_index)
                .clone_from(&get_biome_weights(&biomes, crate::BiomeSurfaceType::Taiga));
            surface_index += 1;
        }
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Chaparral) {}
        if surface_index < 4 && biomes.contains(&crate::BiomeSurfaceType::Rock) {}
        // if surface_index < 4 &&

        self.material
            .get_or_insert_with(|| {
                materials.add(GroundMaterial {
                    noise: self.noise.clone(),
                    dirt: self.dirt.clone(),
                    cobbles: self.cobbles.clone(),
                    texture0: biome_textures[0].clone(),
                    texture1: biome_textures[1].clone(),
                    texture2: biome_textures[2].clone(),
                    texture3: biome_textures[3].clone(),
                    water_color: Srgba::rgb(0.0, 0.1, 0.3).into(),
                    biome_weight,
                    biome_surface,
                })
            })
            .clone()
    }
}

fn get_biome_weights(biomes: &[BiomeSurfaceType; 4], surf: BiomeSurfaceType) -> Vec4 {
    let mut weights = Vec4::ZERO;
    for (i, biome) in biomes.iter().enumerate() {
        if *biome == surf {
            weights[i] = 1.0;
        }
    }
    weights
}
