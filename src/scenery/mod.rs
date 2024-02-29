use bevy::prelude::*;
use precinct_cache::{spawn_precincts, PrecinctCache};

use self::precinct_asset::{PrecinctAsset, PrecinctAssetLoader};

mod msgpack_extension;
mod precinct;
mod precinct_asset;
mod precinct_cache;

pub const PRECINCT_SIZE: i32 = 64;
pub const PRECINCT_SIZE_F: f32 = PRECINCT_SIZE as f32;

pub struct SceneryPlugin;

impl Plugin for SceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrecinctCache::new())
            .register_asset_loader(PrecinctAssetLoader)
            .init_asset::<PrecinctAsset>()
            // .add_plugins((
            //     MaterialPlugin::<GroundMaterial>::default(),
            //     MaterialPlugin::<WaterMaterial>::default(),
            // ))
            // .add_systems(Startup, create_water_material)
            .add_systems(
                Update,
                (
                    spawn_precincts,
                    // gen_ground_meshes,
                    // gen_water_meshes,
                    // gen_flora,
                    // insert_ground_meshes,
                    // insert_water_meshes,
                    // insert_flora,
                    // insert_terrain_maps,
                    // update_terrain_maps,
                    // update_ground_material,
                    // config_textures_modes,
                ),
            );
    }
}
