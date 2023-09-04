use futures_lite::AsyncReadExt;
use serde_repr::{Deserialize_repr, Serialize_repr};
extern crate rmp_serde as rmps;

use std::sync::{Arc, Mutex};

use super::{square::SquareArray, PARCEL_SIZE};
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    math::IRect,
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

const HEIGHT_STRIDE: usize = (PARCEL_SIZE + 1) as usize;
const FLORA_STRIDE: usize = PARCEL_SIZE as usize;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Default, Copy, Clone)]
#[repr(u8)]
pub enum FloraType {
    #[default]
    None = 0,
    RandomTree = 1,
    RandomShrub = 2,
    RandomHerb = 3,
}

impl FloraType {
    fn from_u8(val: u8) -> FloraType {
        match val {
            0 => FloraType::None,
            1 => FloraType::RandomTree,
            2 => FloraType::RandomShrub,
            3 => FloraType::RandomHerb,
            _ => panic!("Invalid flora type"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TerrainContourSer {
    pub id: usize,

    pub height: serde_bytes::ByteBuf,

    pub flora: serde_bytes::ByteBuf,

    // public needsUpdateVertices = false;
    #[serde(alias = "hasTerrain")]
    pub has_terrain: bool,

    #[serde(alias = "hasWater")]
    pub has_water: bool,
}

pub struct TerrainContour {
    pub id: usize,
    pub height: SquareArray<i8>,
    pub flora: SquareArray<FloraType>,
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

pub struct TerrainContoursTable {
    /// Array of shapes, in the order that they appear in the editor's shape catalog.
    shapes: Vec<TerrainContour>,

    /// Array, indexed by shape id, which returns the index of the shape in the shapes list.
    by_id: Vec<usize>,
}

impl TerrainContoursTable {
    /// Get a reference to a terrain shape by it's id.
    pub fn get(&self, id: usize) -> &TerrainContour {
        assert!(id < self.by_id.len());
        &self.shapes[self.by_id[id]]
    }

    /// Get a mutable reference to a terrain shape by it's id.
    pub fn _get_mut(&mut self, id: usize) -> &TerrainContour {
        assert!(id < self.by_id.len());
        &mut self.shapes[self.by_id[id]]
    }
}

#[derive(TypePath, Asset)]
pub struct TerrainContoursTableAsset(pub Arc<Mutex<TerrainContoursTable>>);

#[derive(Default)]
pub struct TerrainContoursTableLoader;

impl AssetLoader for TerrainContoursTableLoader {
    type Asset = TerrainContoursTableAsset;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let shapes_ser: Vec<TerrainContourSer> =
                rmps::from_slice(&bytes).expect("unable to decode terrain shape");
            let mut res = TerrainContoursTable {
                shapes: Vec::with_capacity(shapes_ser.len()),
                by_id: Vec::with_capacity(shapes_ser.len()),
            };

            for (index, shape) in shapes_ser.iter().enumerate() {
                let mut sh = TerrainContour {
                    id: shape.id,
                    height: SquareArray::<i8>::new(HEIGHT_STRIDE, 0),
                    flora: SquareArray::<FloraType>::new(FLORA_STRIDE, FloraType::None),
                    has_terrain: shape.has_terrain,
                    has_water: shape.has_water,
                };
                let mut height: Vec<i8> = vec![0; HEIGHT_STRIDE * HEIGHT_STRIDE];
                for i in 0..HEIGHT_STRIDE * HEIGHT_STRIDE {
                    height[i] = shape.height[i] as i8;
                }
                sh.height.from_slice(height.as_slice());
                let mut flora = Vec::<FloraType>::with_capacity(shape.flora.len());
                flora.resize(shape.flora.len(), FloraType::None);
                for i in 0..shape.flora.len() {
                    flora[i] = FloraType::from_u8(shape.flora[i]);
                }
                flora.resize(shape.flora.len(), FloraType::None);
                sh.flora.from_slice(flora.as_slice());
                res.shapes.push(sh);
                if res.by_id.len() <= shape.id {
                    res.by_id.resize(shape.id + 1, 0);
                }
                res.by_id[shape.id] = index;
            }

            Ok(TerrainContoursTableAsset(Arc::new(Mutex::new(res))))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["contours"]
    }
}

#[derive(Resource)]
pub struct TerrainContoursHandle(pub Handle<TerrainContoursTableAsset>);

impl FromWorld for TerrainContoursHandle {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        TerrainContoursHandle(server.load("terrain/terrain.contours"))
    }
}

const PARCEL_BOUNDS: IRect = IRect {
    min: IVec2 { x: 0, y: 0 },
    max: IVec2 {
        x: PARCEL_SIZE + 1,
        y: PARCEL_SIZE + 1,
    },
};
