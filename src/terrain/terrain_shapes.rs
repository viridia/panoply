extern crate rmp_serde as rmps;

use std::sync::{Arc, Mutex};

use super::{square::SquareArray, PARCEL_SIZE};
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

const HEIGHT_STRIDE: usize = (PARCEL_SIZE + 1) as usize;

const FLORA_STRIDE: usize = PARCEL_SIZE as usize;

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

pub struct TerrainShape {
    pub id: usize,

    pub height: SquareArray<i8>,

    pub flora: SquareArray<u8>,

    // public readonly lakes: Box2[] = [];
    // public needsUpdateVertices = false;
    pub has_terrain: bool,

    pub has_water: bool,
}

// impl TerrainShape {
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
// }

pub struct TerrainShapesTable {
    /// Array of shapes, in the order that they appear in the editor's shape catalog.
    shapes: Vec<TerrainShape>,

    /// Array, indexed by shape id, which returns the index of the shape in the shapes list.
    by_id: Vec<usize>,
}

impl TerrainShapesTable {
    /// Get a reference to a terrain shape by it's id.
    pub fn get(&self, id: usize) -> &TerrainShape {
        assert!(id < self.by_id.len());
        &self.shapes[self.by_id[id]]
    }

    /// Get a mutable reference to a terrain shape by it's id.
    pub fn _get_mut(&mut self, id: usize) -> &TerrainShape {
        assert!(id < self.by_id.len());
        &mut self.shapes[self.by_id[id]]
    }
}

#[derive(TypeUuid, TypePath)]
#[uuid = "059f4368-4ad1-48f4-9151-9b75be1ebfb6"]
pub struct TerrainShapesAsset(pub Arc<Mutex<TerrainShapesTable>>);

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
            let mut res = TerrainShapesTable {
                shapes: Vec::with_capacity(shapes_ser.len()),
                by_id: Vec::with_capacity(shapes_ser.len()),
            };

            for (index, shape) in shapes_ser.iter().enumerate() {
                let mut sh = TerrainShape {
                    id: shape.id,
                    height: SquareArray::<i8>::new(HEIGHT_STRIDE, 0),
                    flora: SquareArray::<u8>::new(FLORA_STRIDE, 0),
                    has_terrain: shape.has_terrain,
                    has_water: shape.has_water,
                };
                let mut height: Vec<i8> = vec![0; HEIGHT_STRIDE * HEIGHT_STRIDE];
                for i in 0..HEIGHT_STRIDE * HEIGHT_STRIDE {
                    height[i] = shape.height[i] as i8;
                }
                sh.height.from_slice(height.as_slice());
                sh.flora.from_slice(shape.flora.as_slice());
                res.shapes.push(sh);
                if res.by_id.len() <= shape.id {
                    res.by_id.resize(shape.id + 1, 0);
                }
                res.by_id[shape.id] = index;
            }

            load_context.set_default_asset(LoadedAsset::new(TerrainShapesAsset(Arc::new(
                Mutex::new(res),
            ))));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["contours"]
    }
}

#[derive(Resource, Default)]
pub struct TerrainShapesHandle(pub Handle<TerrainShapesAsset>);

pub fn load_terrain_shapes(server: Res<AssetServer>, mut handle: ResMut<TerrainShapesHandle>) {
    handle.0 = server.load("terrain/terrain.contours");
}
