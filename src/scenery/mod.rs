use bevy::{prelude::*, utils::HashMap};
use precinct_cache::{spawn_precincts, PrecinctCache};

use self::{
    floor_aspect::{FloorNav, FloorSurface},
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

pub struct SceneryPlugin;

impl Plugin for SceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrecinctCache::new())
            .register_asset_loader(PrecinctAssetLoader)
            .init_asset::<PrecinctAsset>()
            .register_type::<FloorSurface>()
            .register_type::<FloorNav>()
            .register_type::<Vec<String>>()
            .register_type::<HashMap<String, String>>()
            // .add_plugins((
            //     MaterialPlugin::<GroundMaterial>::default(),
            //     MaterialPlugin::<WaterMaterial>::default(),
            // ))
            // .add_systems(Startup, create_water_material)
            .add_systems(Update, (spawn_precincts, read_precinct_data));
    }
}
