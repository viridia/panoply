use bevy::{prelude::*, utils::HashMap};
use precinct_cache::{spawn_precincts, PrecinctCache};

use self::{
    floor_aspect::{FloorGeometry, FloorNav, NoiseFloorSurface, StdFloorSurface},
    floor_mesh::{
        gen_floor_meshes, insert_floor_meshes, rebuild_floor_materials, update_floor_aspects,
    },
    precinct::read_precinct_data,
    precinct_asset::{PrecinctAsset, PrecinctAssetLoader},
};

pub mod floor_aspect;
mod floor_mesh;
mod floor_region;
mod msgpack_extension;
mod precinct;
mod precinct_asset;
mod precinct_cache;

pub const PRECINCT_SIZE: i32 = 64;
pub const PRECINCT_SIZE_F: f32 = PRECINCT_SIZE as f32;

pub const FLOOR_THICKNESS: f32 = 0.2; // Thickness of floors
pub const PHYSICS_FLOOR_THICKNESS: f32 = 0.1; // Thickness of floor colliders
pub const TIER_OFFSET: f32 = 0.02 - 2.; // Tiers are slightly higher than the terrain.

pub struct SceneryPlugin;

impl Plugin for SceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrecinctCache::new())
            .register_asset_loader(PrecinctAssetLoader)
            .init_asset::<PrecinctAsset>()
            .register_type::<StdFloorSurface>()
            .register_type::<NoiseFloorSurface>()
            .register_type::<FloorGeometry>()
            .register_type::<FloorNav>()
            .register_type::<Vec<String>>()
            .register_type::<HashMap<String, String>>()
            .add_systems(
                Update,
                (
                    spawn_precincts,
                    (
                        read_precinct_data,
                        update_floor_aspects,
                        apply_deferred,
                        gen_floor_meshes,
                    )
                        .chain(),
                    insert_floor_meshes,
                    rebuild_floor_materials,
                ),
            );
    }
}
