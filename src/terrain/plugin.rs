use super::{
    cleanup_parcel_physics,
    flora::{gen_flora, insert_flora, spawn_flora_model_instances},
    gen_ground_meshes, insert_ground_meshes, spawn_parcels,
    terrain_map::{
        insert_terrain_maps, update_terrain_maps, TerrainMapAsset, TerrainMapLoader,
        TerrainMapsHandleResource,
    },
    water_mesh::{gen_water_meshes, insert_water_meshes},
    ParcelCache,
};
use bevy::prelude::*;
use panoply_terrain::PanoplyTerrainPlugin;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanoplyTerrainPlugin);
        app.insert_resource(ParcelCache::new())
            .register_asset_loader(TerrainMapLoader)
            .init_asset::<TerrainMapAsset>()
            .init_resource::<TerrainMapsHandleResource>()
            .add_systems(Startup, cleanup_parcel_physics)
            .add_systems(
                Update,
                (
                    spawn_parcels,
                    gen_ground_meshes.after(spawn_parcels),
                    gen_water_meshes.after(spawn_parcels),
                    gen_flora.after(spawn_parcels),
                    insert_ground_meshes,
                    insert_water_meshes,
                    insert_flora,
                    insert_terrain_maps,
                    update_terrain_maps,
                    spawn_flora_model_instances,
                ),
            );
    }
}
