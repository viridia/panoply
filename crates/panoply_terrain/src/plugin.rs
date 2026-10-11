use crate::{
    ground_material::{GroundMaterial, GroundMaterialCache},
    water_material::{create_water_material, WaterMaterialResource},
    BiomesAsset, BiomesHandle, BiomesLoader, TerrainContoursHandle, TerrainContoursTableAsset,
    TerrainContoursTableLoader, TerrainTypes, WaterMaterial,
};
use bevy::{
    asset::embedded_asset,
    prelude::*,
};

// use super::{
//     cleanup_parcel_physics,
//     flora::{gen_flora, insert_flora, spawn_flora_model_instances},
//     gen_ground_meshes,
//     // ground_material::GroundMaterial,
//     ground_material_new::GroundMaterialNew,
//     insert_ground_meshes,
//     spawn_parcels,
//     terrain_map::{
//         insert_terrain_maps, update_terrain_maps, TerrainMapAsset,
//         TerrainMapLoader, TerrainMapsHandleResource,
//     },
//     water_material::{create_water_material, WaterMaterial, WaterMaterialResource},
//     water_mesh::{gen_water_meshes, insert_water_meshes},
//     ParcelCache,
// };
// use panoply_terrain::{
//     TerrainContoursHandle, TerrainContoursTableAsset, TerrainContoursTableLoader,
// };

pub struct PanoplyTerrainPlugin;

impl Plugin for PanoplyTerrainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/ground.wesl");
        embedded_asset!(app, "shaders/water.wesl");

        app
        // .insert_resource(ParcelCache::new())
            .register_asset_loader(TerrainContoursTableLoader)
            // .register_asset_loader(TerrainMapLoader)
            .register_asset_loader(BiomesLoader)
            .register_type::<TerrainTypes>()
            .init_asset::<TerrainContoursTableAsset>()
            // .init_asset::<TerrainMapAsset>()
            .init_asset::<BiomesAsset>()
            .init_resource::<BiomesHandle>()
            .init_resource::<TerrainContoursHandle>()
            .init_resource::<GroundMaterialCache>()
            // .init_resource::<TerrainMapsHandleResource>()
            .init_resource::<WaterMaterialResource>()
            .init_resource::<crate::HemisphereLight>()
            .add_plugins((
                MaterialPlugin::<GroundMaterial>::default(),
                MaterialPlugin::<WaterMaterial>::default(),
            ))
            .add_systems(Startup, create_water_material)
            .add_systems(PostUpdate, crate::hemisphere::sync_hemisphere_light)
            // .add_systems(Startup, (cleanup_parcel_physics))
            // .add_systems(
            //     Update,
            //     (
            //         spawn_parcels,
            //         gen_ground_meshes,
            //         gen_water_meshes,
            //         gen_flora,
            //         insert_ground_meshes,
            //         insert_water_meshes,
            //         insert_flora,
            //         insert_terrain_maps,
            //         update_terrain_maps,
            //         spawn_flora_model_instances,
            //     ),
            // )
            ;
    }
}
