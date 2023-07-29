#![allow(dead_code)]
mod biome;
mod ground_material;
mod ground_mesh;
mod parcel;
mod parcel_cache;
mod plugin;
mod square;
mod terrain_map;
mod terrain_shapes;

pub const PARCEL_SIZE: i32 = 16;
pub const PARCEL_SIZE_F: f32 = PARCEL_SIZE as f32;

pub const PARCEL_MESH_RESOLUTION: i32 = 64;
pub const PARCEL_MESH_STRIDE: i32 = PARCEL_MESH_RESOLUTION + 1;
pub const PARCEL_MESH_VERTEX_COUNT: usize = (PARCEL_MESH_STRIDE * PARCEL_MESH_STRIDE) as usize;
pub const PARCEL_MESH_SCALE: f32 = PARCEL_SIZE as f32 / PARCEL_MESH_RESOLUTION as f32;

pub use ground_mesh::*;
pub use parcel_cache::*;
pub use plugin::*;
