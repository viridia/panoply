use bevy::{
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    platform::collections::HashMap,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        VertexFormat,
    },
    shader::ShaderRef,
};

use crate::{load_tiling_texture, BiomeSurfaceType, BiomesTable, ParcelBiomes};

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct BiomeSurfaceAttrs {
    /// Surface roughness.
    roughness: f32,
    /// How far noise moves this surface's boundary: higher is clumpier and spreads the
    /// transition wider. Scaled down to nothing where the surface's weight nears 0 or 1.
    blend_var: f32,
    /// The surface starts to appear where its weight plus noise passes this value.
    blend_t0: f32,
    /// The surface is fully opaque above this value. The gap from `blend_t0` is the softness of
    /// its edge, and their midpoint sets its coverage: lower covers more.
    blend_t1: f32,
    /// As `blend_var`, for the tinted rim.
    edge_var: f32,
    /// Below this value the surface takes the full `edge_tint`.
    edge_t0: f32,
    /// Above this value the surface has no tint, so the rim lies between `blend_t0` and here.
    edge_t1: f32,
    /// Texture repeats per metre.
    tx_scale: f32,
    /// Multiplies the surface's color at its rim, near roads and other surfaces.
    edge_tint: LinearRgba,
}

// Blend and edge thresholds from the old engine's terrain shader (faery's `TerrainMaterial.ts`),
// one set per surface; its moss blend, a sharper curve, is approximated.
impl BiomeSurfaceAttrs {
    const BASE: BiomeSurfaceAttrs = BiomeSurfaceAttrs {
        roughness: 0.99,
        blend_var: 0.5,
        blend_t0: 0.4,
        blend_t1: 0.6,
        edge_var: 0.5,
        edge_t0: 0.45,
        edge_t1: 0.7,
        tx_scale: 0.2,
        edge_tint: LinearRgba::new(0.65, 0.65, 0.65, 1.0),
    };

    pub const GRASS: BiomeSurfaceAttrs = BiomeSurfaceAttrs {
        edge_t0: 0.5,
        ..Self::BASE
    };

    pub const MOSS: BiomeSurfaceAttrs = BiomeSurfaceAttrs {
        blend_var: 0.9,
        blend_t0: 0.04,
        blend_t1: 0.56,
        edge_var: 0.9,
        edge_t1: 0.65,
        ..Self::BASE
    };

    pub const SAND: BiomeSurfaceAttrs = Self::BASE;

    // Snow lies in clumps rather than fading: much noise over a narrow threshold, so that where
    // it is thin (a low weight) only the patches remain.
    pub const SNOW: BiomeSurfaceAttrs = BiomeSurfaceAttrs {
        blend_var: 0.7,
        blend_t0: 0.46,
        blend_t1: 0.54,
        edge_var: 0.7,
        edge_t0: 0.52,
        edge_t1: 0.65,
        ..Self::BASE
    };

    pub const TAIGA: BiomeSurfaceAttrs = Self::BASE;
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

    /// Hemisphere light: sky color, then ground color, brightness folded in. Kept up to date by
    /// `sync_hemisphere_light`.
    #[uniform(23)]
    pub hemisphere: [LinearRgba; 2],
}

impl GroundMaterial {
    pub const ATTRIBUTE_TERRAIN_STYLE: MeshVertexAttribute =
        MeshVertexAttribute::new("terrain_style", 200, VertexFormat::Uint32x2);
}

impl Material for GroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://panoply_terrain/shaders/ground.wesl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "embedded://panoply_terrain/shaders/ground.wesl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Position and normal are where Bevy's default prepass shader reads them.
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(3),
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

    /// One material per combination of surfaces at a parcel's four corners, since the material
    /// carries the textures and the weights that blend them across the parcel.
    pub materials: HashMap<[BiomeSurfaceType; 4], Handle<GroundMaterial>>,
}

impl FromWorld for GroundMaterialCache {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        GroundMaterialCache {
            noise: load_tiling_texture(assets, "terrain/textures/noise.ktx2"),
            dirt: load_tiling_texture(assets, "terrain/textures/dirt.ktx2"),
            cobbles: load_tiling_texture(assets, "terrain/textures/cobbles.ktx2"),
            grass: load_tiling_texture(assets, "terrain/textures/grass.ktx2"),
            moss: load_tiling_texture(assets, "terrain/textures/moss.ktx2"),
            sand: load_tiling_texture(assets, "terrain/textures/sand.ktx2"),
            snow: load_tiling_texture(assets, "terrain/textures/snow.ktx2"),
            taiga: load_tiling_texture(assets, "terrain/textures/taiga2.ktx2"),
            materials: HashMap::default(),
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
        if let Some(material) = self.materials.get(&biomes) {
            return material.clone();
        }
        // Each corner's weight for every surface. Tundra is taiga with half snow on top.
        let mut weights: HashMap<BiomeSurfaceType, Vec4> = HashMap::default();
        for (corner, surface) in biomes.iter().enumerate() {
            let mut add = |surface: BiomeSurfaceType, weight: f32| {
                let w = &mut weights.entry(surface).or_insert(Vec4::ZERO)[corner];
                *w = w.max(weight);
            };
            match surface {
                BiomeSurfaceType::Tundra => {
                    add(BiomeSurfaceType::Taiga, 1.0);
                    add(BiomeSurfaceType::Snow, 0.5);
                }
                surface => add(*surface, 1.0),
            }
        }

        // The topcoats over the dirt underlay, in the order they are painted, each covering those
        // before. Chaparral has no texture yet.
        let layers = [
            (
                BiomeSurfaceType::Grass,
                &self.grass,
                BiomeSurfaceAttrs::GRASS,
            ),
            (
                BiomeSurfaceType::Taiga,
                &self.taiga,
                BiomeSurfaceAttrs::TAIGA,
            ),
            (BiomeSurfaceType::Moss, &self.moss, BiomeSurfaceAttrs::MOSS),
            (BiomeSurfaceType::Sand, &self.sand, BiomeSurfaceAttrs::SAND),
            (BiomeSurfaceType::Snow, &self.snow, BiomeSurfaceAttrs::SNOW),
        ];
        // The first topcoat covers the whole parcel, so that the next paints over it rather than
        // over dirt; unless a corner is bare (rock or dirt), whose underlay is then the first.
        let bare = biomes
            .iter()
            .any(|s| matches!(s, BiomeSurfaceType::Rock | BiomeSurfaceType::Dirt));
        let mut biome_textures: [Handle<Image>; 4] = Default::default();
        let mut biome_weight = Mat4::ZERO;
        let mut biome_surface = [BiomeSurfaceAttrs::GRASS; 8];
        let mut surface_index = 0usize;
        for (surface, texture, attrs) in layers {
            let Some(&weight) = weights.get(&surface) else {
                continue;
            };
            if surface_index == 4 {
                break;
            }
            biome_textures[surface_index] = texture.clone();
            biome_surface[surface_index] = attrs;
            *biome_weight.col_mut(surface_index) = if surface_index == 0 && !bare {
                Vec4::ONE
            } else {
                weight
            };
            surface_index += 1;
        }

        let material = materials.add(GroundMaterial {
            noise: self.noise.clone(),
            dirt: self.dirt.clone(),
            cobbles: self.cobbles.clone(),
            texture0: biome_textures[0].clone(),
            texture1: biome_textures[1].clone(),
            texture2: biome_textures[2].clone(),
            texture3: biome_textures[3].clone(),
            water_color: Srgba::rgb(0.0, 0.25, 0.3).into(),
            biome_weight,
            biome_surface,
            hemisphere: [LinearRgba::NONE; 2],
        });
        self.materials.insert(biomes, material.clone());
        material
    }
}
