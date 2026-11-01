use bevy::{asset::Handle, math::Vec2, prelude::*};

use super::floor_surface::FloorSurface;

#[derive(Component, Debug, Clone, Default)]
pub struct FloorRegion {
    /// Floor level
    pub level: i32,

    /// Surface appearance and properties
    pub surface: Handle<FloorSurface>,

    /// Polygonal outline of floor
    pub poly: Vec<Vec2>,

    /// Holes in the polygon.
    pub holes: Vec<Vec<Vec2>>,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildFloorAspects;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildFloorMesh;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildFloorMaterials;
