extern crate rmp_serde as rmps;

use super::PARCEL_SIZE;
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

const HEIGHT_STRIDE: usize = (PARCEL_SIZE + 1) as usize;
const HEIGHT_VERTEX_COUNT: usize = HEIGHT_STRIDE * HEIGHT_STRIDE;

const FLORA_STRIDE: usize = PARCEL_SIZE as usize;
const FLORA_VERTEX_COUNT: usize = FLORA_STRIDE * FLORA_STRIDE;

type HeightArray = [i8; HEIGHT_VERTEX_COUNT];
type FloraArray = [u8; FLORA_VERTEX_COUNT];

#[derive(Serialize, Deserialize, Debug)]
struct TerrainShapeSer {
    pub id: usize,

    pub height: serde_bytes::ByteBuf,

    pub flora: serde_bytes::ByteBuf,

    // public readonly lakes: Box2[] = [];
    // public needsUpdateVertices = false;
    #[serde(alias = "hasTerrain")]
    pub has_terrain: bool,

    #[serde(alias = "hasWater")]
    pub has_water: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TerrainShape {
    pub id: usize,

    #[serde(with = "serde_arrays")]
    pub height: HeightArray,

    #[serde(with = "serde_arrays")]
    pub flora: FloraArray,

    // public readonly lakes: Box2[] = [];
    // public needsUpdateVertices = false;
    #[serde(alias = "hasTerrain")]
    pub has_terrain: bool,

    #[serde(alias = "hasWater")]
    pub has_water: bool,
}

impl TerrainShape {
    /// Get terrain height (non-interpolated).
    pub fn _height_at(&self, x: usize, y: usize) -> i8 {
        assert!(x < HEIGHT_STRIDE);
        assert!(y < HEIGHT_STRIDE);
        return self.height[x + y * HEIGHT_STRIDE];
    }

    pub fn _flora_at(&self, x: usize, y: usize) -> u8 {
        assert!(x < FLORA_STRIDE);
        assert!(y < FLORA_STRIDE);
        return self.flora[x + y * FLORA_STRIDE];
    }

    //   public fillHeight(area: Box2, height: number): TerrainPlot {
    //     const { min, max } = clampBox(area);
    //     height = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, height));
    //     for (let y = min.y; y <= max.y; y += 1) {
    //       const rowOffset = y * PLOT_VERTEX_STRIDE;
    //       this.terrainHeight.fill(height, rowOffset + min.x, rowOffset + max.x + 1);
    //     }
    //     return this;
    //   }

    //   public fillFlora(area: Box2, flora: FloraType): this {
    //     const { min, max } = clampBox(area);
    //     for (let y = min.y; y < max.y; y += 1) {
    //       const rowOffset = y * PLOT_LENGTH;
    //       this.flora.fill(flora, rowOffset + min.x, rowOffset + max.x);
    //     }
    //     return this;
    //   }
}

#[derive(Debug, Deserialize, TypeUuid, TypePath)]
#[uuid = "002b890e-ea0d-43ba-a19d-c44438c8e9e2"]
pub struct TerrainShapesResource {
    /// Array of shapes, in the order that they appear in the editor's shape catalog.
    shapes: Vec<TerrainShape>,

    /// Array, indexed by shape id, which returns the index of the shape in the shapes list.
    by_id: Vec<usize>,
}

impl TerrainShapesResource {
    /// Get a reference to a terrain shape by it's id.
    pub fn _get(&self, id: usize) -> &TerrainShape {
        assert!(id < self.by_id.len());
        &self.shapes[self.by_id[id]]
    }

    /// Get a mutable reference to a terrain shape by it's id.
    pub fn _get_mut(&mut self, id: usize) -> &TerrainShape {
        assert!(id < self.by_id.len());
        &mut self.shapes[self.by_id[id]]
    }
}

/// Repository of terrain shapes
#[derive(Resource, Default)]
pub struct TerrainShapes(pub Handle<TerrainShapesResource>);

#[derive(Default)]
pub struct TerrainShapesLoader;

impl AssetLoader for TerrainShapesLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let shapes_ser: Vec<TerrainShapeSer> =
                rmps::from_slice(bytes).expect("unable to decode terrain shape");
            let mut res = TerrainShapesResource {
                shapes: Vec::with_capacity(shapes_ser.len()),
                by_id: Vec::with_capacity(shapes_ser.len()),
            };

            for (index, shape) in shapes_ser.iter().enumerate() {
                let mut sh = TerrainShape {
                    id: shape.id,
                    height: [0; HEIGHT_VERTEX_COUNT],
                    flora: [0; FLORA_VERTEX_COUNT],
                    has_terrain: shape.has_terrain,
                    has_water: shape.has_water,
                };
                for i in 0..HEIGHT_VERTEX_COUNT {
                    sh.height[i] = shape.height[i] as i8;
                }
                sh.flora.copy_from_slice(shape.flora.as_slice());
                res.shapes.push(sh);
                if res.by_id.len() <= shape.id {
                    res.by_id.resize(shape.id + 1, 0);
                }
                res.by_id[shape.id] = index;
            }

            load_context.set_default_asset(LoadedAsset::new(res));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tsh.msgpack"]
    }
}

pub fn load_terrain_shapes(
    mut terrain_shapes: ResMut<TerrainShapes>,
    asset_server: Res<AssetServer>,
) {
    terrain_shapes.0 = asset_server.load("terrain.tsh.msgpack");
}

// pub fn terrain_shapes_loaded(
//     mut terrain_shapes: ResMut<TerrainShapes>,
//     shape_table: ResMut<Assets<TerrainShapesTable>>,
// ) {
//     // println!("Loading terrain shapes");
//     if !terrain_shapes.loaded {
//         let shapes = shape_table.get(&terrain_shapes.shapes);
//         if shapes.is_some() {
//             for (index, shape) in shapes.unwrap().0.iter().enumerate() {
//                 terrain_shapes.by_index[shape.id] = index;
//             }
//             terrain_shapes.loaded = true;
//         }
//     }
// }