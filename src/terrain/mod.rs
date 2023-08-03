#![allow(dead_code)]
mod biome;
mod flora;
mod ground_material;
mod ground_mesh;
mod parcel;
mod parcel_cache;
mod plugin;
mod rotator;
mod square;
mod terrain_contours;
mod terrain_map;
mod water_material;
mod water_mesh;

pub const PARCEL_SIZE: i32 = 16;
pub const PARCEL_SIZE_F: f32 = PARCEL_SIZE as f32;

pub const PARCEL_WATER_RESOLUTION: usize = 32;
pub const PARCEL_WATER_RESOLUTION_S: i32 = 32;
pub const PARCEL_WATER_STRIDE: usize = PARCEL_WATER_RESOLUTION + 1;
pub const PARCEL_WATER_VERTEX_COUNT: usize = PARCEL_WATER_STRIDE * PARCEL_WATER_STRIDE;

pub const PARCEL_MESH_RESOLUTION: i32 = 64;
pub const PARCEL_MESH_STRIDE: i32 = PARCEL_MESH_RESOLUTION + 1;
pub const PARCEL_MESH_VERTEX_COUNT: usize = (PARCEL_MESH_STRIDE * PARCEL_MESH_STRIDE) as usize;
pub const PARCEL_MESH_SCALE: f32 = PARCEL_SIZE as f32 / PARCEL_MESH_RESOLUTION as f32;

pub use ground_mesh::*;
pub use parcel_cache::*;
pub use plugin::*;
