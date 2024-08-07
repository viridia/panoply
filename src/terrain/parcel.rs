use bevy::prelude::*;

use crate::terrain::{PARCEL_TERRAIN_FX_SIZE, PARCEL_TERRAIN_FX_STRIDE};

use super::{TerrainFxVertexAttr, PARCEL_TERRAIN_FX_AREA};
#[derive(Eq, PartialEq, Hash)]
pub struct ParcelKey {
    pub realm: Entity,
    pub x: i32,
    pub z: i32,
}

pub const ADJACENT_COUNT: usize = 9;
pub const CENTER_SHAPE: usize = 4;

// A reference to a terrain shape
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub struct ShapeRef {
    pub shape: u16,
    pub rotation: u8,
}

impl ShapeRef {
    pub fn new() -> ShapeRef {
        Self {
            shape: 0,
            rotation: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct ParcelTerrainFx(pub [TerrainFxVertexAttr; PARCEL_TERRAIN_FX_AREA]);

impl ParcelTerrainFx {
    #[inline(always)]
    pub fn get(&self, x: usize, z: usize) -> TerrainFxVertexAttr {
        assert!(x < PARCEL_TERRAIN_FX_SIZE);
        assert!(z < PARCEL_TERRAIN_FX_SIZE);
        self.0[x + z * PARCEL_TERRAIN_FX_STRIDE]
    }
}

#[derive(Component)]
pub struct Parcel {
    pub realm: Entity,
    pub coords: IVec2,
    pub visible: bool,
    pub contours: [ShapeRef; ADJACENT_COUNT],

    /// Biome ids assigned to each corner.
    pub biomes: [u8; 4],

    /// Entity that represents the ground mesh of this parcel.
    pub ground_entity: Option<Entity>,

    /// Entity that represents the water mesh of this parcel.
    pub water_entity: Option<Entity>,

    /// Entity that contains the flora instances for this parcel.
    pub flora_entity: Option<Entity>,

    /// Terrain effects for this parcel.
    pub terrain_fx: ParcelTerrainFx,
}

impl Parcel {
    pub fn center_shape(&self) -> ShapeRef {
        self.contours[CENTER_SHAPE]
    }

    pub fn has_shape(&self, shape: u16) -> bool {
        self.contours.iter().any(|&s| s.shape == shape)
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildParcelTerrainFx;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildParcelGroundMesh;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildParcelPhysics;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ParcelWaterChanged;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ParcelFloraChanged;

#[derive(Component)]
pub struct ParcelThumbnail;
