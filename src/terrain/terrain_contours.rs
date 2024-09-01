use futures_lite::{AsyncReadExt, AsyncWriteExt};
use serde_repr::{Deserialize_repr, Serialize_repr};
extern crate rmp_serde as rmps;
use std::sync::{Arc, RwLock};

use super::{square::SquareArray, PARCEL_HEIGHT_SCALE, PARCEL_SIZE, PARCEL_SIZE_U};
use bevy::{
    asset::{
        io::{AssetWriterError, Reader},
        saver::AssetSaver,
        AssetLoader, LoadContext,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainContour {
    pub id: usize,

    #[serde(
        serialize_with = "serialize_height_array",
        deserialize_with = "deserialize_height_array"
    )]
    pub height: SquareArray<i8>,

    #[serde(
        serialize_with = "serialize_flora_array",
        deserialize_with = "deserialize_flora_array"
    )]
    pub flora: SquareArray<FloraType>,

    #[serde(alias = "hasTerrain")]
    pub has_terrain: bool,
    #[serde(alias = "hasWater")]
    pub has_water: bool,
}

fn deserialize_height_array<'de, D>(deserializer: D) -> Result<SquareArray<i8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let data = serde_bytes::ByteBuf::deserialize(deserializer)?;
    let mut res = SquareArray::<i8>::new(HEIGHT_STRIDE, 0);
    assert_eq!(data.len(), HEIGHT_STRIDE * HEIGHT_STRIDE);
    for i in 0..data.len() {
        res.set(i % HEIGHT_STRIDE, i / HEIGHT_STRIDE, data[i] as i8);
    }
    Ok(res)
}

fn serialize_height_array<S>(data: &SquareArray<i8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut bytes = Vec::<u8>::with_capacity(HEIGHT_STRIDE * HEIGHT_STRIDE);
    for i in 0..HEIGHT_STRIDE * HEIGHT_STRIDE {
        bytes.push(data.get(i % HEIGHT_STRIDE, i / HEIGHT_STRIDE) as u8);
    }
    serializer.serialize_bytes(bytes.as_ref())
}

fn deserialize_flora_array<'de, D>(deserializer: D) -> Result<SquareArray<FloraType>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let data = serde_bytes::ByteBuf::deserialize(deserializer)?;
    let mut res = SquareArray::<FloraType>::new(FLORA_STRIDE, FloraType::None);
    assert_eq!(data.len(), FLORA_STRIDE * FLORA_STRIDE);
    for i in 0..data.len() {
        res.set(
            i % FLORA_STRIDE,
            i / FLORA_STRIDE,
            FloraType::from_u8(data[i]),
        );
    }
    Ok(res)
}

fn serialize_flora_array<S>(data: &SquareArray<FloraType>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut res = Vec::<u8>::with_capacity(FLORA_STRIDE * FLORA_STRIDE);
    for i in 0..FLORA_STRIDE * FLORA_STRIDE {
        res.push(data.get(i % FLORA_STRIDE, i / FLORA_STRIDE) as u8);
    }
    serializer.serialize_bytes(res.as_ref())
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

    pub fn clone_from(&mut self, other: &TerrainContour) {
        self.height.copy_from_slice(other.height.elts());
        self.flora.copy_from_slice(other.flora.elts());
        self.has_terrain = other.has_terrain;
        self.has_water = other.has_water;
    }
}

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

#[derive(TypePath, Asset, Default, Clone)]
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
        let shapes: Vec<TerrainContour> =
            rmps::from_slice(&bytes).expect("unable to decode terrain shape");
        let num_shapes = shapes.len();
        let mut res = TerrainContoursTable {
            shapes,
            by_id: Vec::with_capacity(num_shapes),
        };

        for (index, shape) in res.shapes.iter().enumerate() {
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

pub struct TerrainContoursTableSaver;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TerrainContoursSaverError {
    #[error("Could not save terrain contours: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not encode terrain contours: {0}")]
    Encode(#[from] rmps::encode::Error),
    #[error("Could not commit terrain contours file: {0}")]
    Commit(#[from] AssetWriterError),
}

impl TerrainContoursTableSaver {
    pub fn encode(
        &self,
        asset: &TerrainContoursTableAsset,
    ) -> Result<Vec<u8>, rmps::encode::Error> {
        let table = asset.0.read().unwrap();
        rmps::encode::to_vec_named(&table.shapes)
    }
}

impl AssetSaver for TerrainContoursTableSaver {
    type Asset = TerrainContoursTableAsset;
    type Settings = ();
    type OutputLoader = TerrainContoursTableLoader;
    type Error = TerrainContoursSaverError;

    async fn save<'a>(
        &'a self,
        writer: &'a mut bevy::asset::io::Writer,
        asset: bevy::asset::saver::SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> Result<(), Self::Error> {
        let v = self.encode(&asset)?;
        writer.write_all(&v).await?;
        Ok(())
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
