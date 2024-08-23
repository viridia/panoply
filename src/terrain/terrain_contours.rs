use futures_lite::AsyncReadExt;
use serde_repr::{Deserialize_repr, Serialize_repr};
extern crate rmp_serde as rmps;

use std::sync::{Arc, RwLock};

use super::{square::SquareArray, PARCEL_HEIGHT_SCALE, PARCEL_SIZE, PARCEL_SIZE_U};
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    math::IRect,
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const HEIGHT_STRIDE: usize = PARCEL_SIZE_U + 1;
const FLORA_STRIDE: usize = PARCEL_SIZE_U;

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

impl TerrainContour {
    /// Get the height at a given point in the terrain contour.
    pub fn height_at(&self, x: usize, y: usize, rotation: u8) -> f32 {
        let (xr, yr) = match rotation {
            0 => (x, y),
            1 => (y, PARCEL_SIZE_U - x),
            2 => (PARCEL_SIZE_U - x, PARCEL_SIZE_U - y),
            3 => (PARCEL_SIZE_U - y, x),
            _ => panic!("Invalid rotation"),
        };
        self.height.get(xr, yr) as f32 * PARCEL_HEIGHT_SCALE
    }

    /// Get the height at a given point in the terrain contour (unscaled).
    pub fn unscaled_height_at(&self, x: usize, y: usize, rotation: u8) -> i32 {
        let (xr, yr) = match rotation {
            0 => (x, y),
            1 => (y, PARCEL_SIZE_U - x),
            2 => (PARCEL_SIZE_U - x, PARCEL_SIZE_U - y),
            3 => (PARCEL_SIZE_U - y, x),
            _ => panic!("Invalid rotation"),
        };
        self.height.get(xr, yr) as i32
    }

    /// Set the height at a given point in the terrain contour, accounting for rotation.
    pub fn set_height_at(&mut self, x: usize, y: usize, rotation: u8, value: i8) {
        let (xr, yr) = match rotation {
            0 => (x, y),
            1 => (y, PARCEL_SIZE_U - x),
            2 => (PARCEL_SIZE_U - x, PARCEL_SIZE_U - y),
            3 => (PARCEL_SIZE_U - y, x),
            _ => panic!("Invalid rotation"),
        };
        self.height.set(xr, yr, value)
    }

    /// Set the flora type at a given point in the terrain contour, accounting for rotation.
    pub fn set_flora_at(&mut self, x: usize, y: usize, rotation: u8, value: FloraType) {
        let (xr, yr) = match rotation {
            0 => (x, y),
            1 => (y, PARCEL_SIZE_U - 1 - x),
            2 => (PARCEL_SIZE_U - 1 - x, PARCEL_SIZE_U - 1 - y),
            3 => (PARCEL_SIZE_U - 1 - y, x),
            _ => panic!("Invalid rotation"),
        };
        self.flora.set(xr, yr, value);
    }
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

#[derive(Default)]
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
    pub fn get_mut(&mut self, id: usize) -> &mut TerrainContour {
        assert!(id < self.by_id.len());
        &mut self.shapes[self.by_id[id]]
    }

    /// List all terrain shapes.
    pub fn list(&self) -> &[TerrainContour] {
        &self.shapes
    }
}

#[derive(TypePath, Asset, Default)]
pub struct TerrainContoursTableAsset(pub Arc<RwLock<TerrainContoursTable>>);

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TerrainContoursLoaderError {
    #[error("Could not load terrain contours: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct TerrainContoursTableLoader;

impl AssetLoader for TerrainContoursTableLoader {
    type Asset = TerrainContoursTableAsset;
    type Error = TerrainContoursLoaderError;
    type Settings = ();

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
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
            #[allow(clippy::needless_range_loop)]
            for i in 0..HEIGHT_STRIDE * HEIGHT_STRIDE {
                height[i] = shape.height[i] as i8;
            }
            sh.height.copy_from_slice(height.as_slice());
            let mut flora = Vec::<FloraType>::with_capacity(shape.flora.len());
            flora.resize(shape.flora.len(), FloraType::None);
            for i in 0..shape.flora.len() {
                flora[i] = FloraType::from_u8(shape.flora[i]);
            }
            flora.resize(shape.flora.len(), FloraType::None);
            sh.flora.copy_from_slice(flora.as_slice());
            res.shapes.push(sh);
            if res.by_id.len() <= shape.id {
                res.by_id.resize(shape.id + 1, 0);
            }
            res.by_id[shape.id] = index;
        }

        Ok(TerrainContoursTableAsset(Arc::new(RwLock::new(res))))
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
