use crate::{
    cleanup_parcel_physics,
    flora::{gen_flora, insert_flora, spawn_flora_model_instances},
    gen_ground_meshes,
    ground_material::{GroundMaterial, GroundMaterialCache},
    insert_ground_meshes, spawn_parcels,
    terrain_map::{
        insert_terrain_maps, update_terrain_maps, TerrainMapAsset, TerrainMapLoader,
        TerrainMapsHandleResource,
    },
    water_material::{create_water_material, WaterMaterialResource},
    water_mesh::{gen_water_meshes, insert_water_meshes},
    BiomesAsset, BiomesHandle, BiomesLoader, ParcelCache, TerrainContoursHandle,
    TerrainContoursTableAsset, TerrainContoursTableLoader, TerrainTypes, WaterMaterial,
};
use bevy::{asset::embedded_asset, prelude::*};

pub struct PanoplyTerrainPlugin;

impl Plugin for PanoplyTerrainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/ground.wesl");
        embedded_asset!(app, "shaders/water.wesl");

        app.init_resource::<ParcelCache>()
            .register_asset_loader(TerrainContoursTableLoader)
            .register_asset_loader(TerrainMapLoader)
            .register_asset_loader(BiomesLoader)
            .register_type::<TerrainTypes>()
            .init_asset::<TerrainContoursTableAsset>()
            .init_asset::<TerrainMapAsset>()
            .init_asset::<BiomesAsset>()
            .init_resource::<BiomesHandle>()
            .init_resource::<TerrainContoursHandle>()
            .init_resource::<GroundMaterialCache>()
            .init_resource::<TerrainMapsHandleResource>()
            .init_resource::<WaterMaterialResource>()
            .init_resource::<crate::HemisphereLight>()
            .add_plugins((
                MaterialPlugin::<GroundMaterial>::default(),
                MaterialPlugin::<WaterMaterial>::default(),
            ))
            .add_systems(Startup, (create_water_material, cleanup_parcel_physics))
            .add_systems(PostUpdate, crate::hemisphere::sync_hemisphere_light)
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
