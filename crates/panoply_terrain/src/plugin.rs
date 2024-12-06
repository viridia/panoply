use crate::{
    ground_material::{GroundMaterial, GroundMaterialCache},
    water_material::{create_water_material, WaterMaterialResource},
    BiomesAsset, BiomesHandle, BiomesLoader, TerrainContoursHandle, TerrainContoursTableAsset,
    TerrainContoursTableLoader, TerrainTypes, WaterMaterial,
};
use bevy::{
    asset::embedded_asset,
    // image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
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
        embedded_asset!(app, "shaders/ground.wgsl");
        embedded_asset!(app, "shaders/water.wgsl");

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
            .add_plugins((
                MaterialPlugin::<GroundMaterial>::default(),
                MaterialPlugin::<WaterMaterial>::default(),
            ))
            .add_systems(Startup, create_water_material)
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
            //         config_textures_modes,
            //         spawn_flora_model_instances,
            //     ),
            // )
            ;
    }
}

// pub fn config_textures_modes(
//     server: Res<AssetServer>,
//     mut assets: ResMut<Assets<Image>>,
//     mut ev_image: EventReader<AssetEvent<Image>>,
// ) {
//     for ev in ev_image.read() {
//         if let AssetEvent::Added { id } = ev {
//             if let Some(asset_path) = server.get_path(*id) {
//                 let path = asset_path.path();
//                 if path.parent().expect("path").to_str().expect("path") == "textures" {
//                     if let Some(image) = assets.get_mut(*id) {
//                         image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
//                             label: Some("Terrain textures".to_string()),
//                             address_mode_u: ImageAddressMode::Repeat,
//                             address_mode_v: ImageAddressMode::ClampToEdge,
//                             address_mode_w: ImageAddressMode::ClampToEdge,
//                             mag_filter: ImageFilterMode::Linear,
//                             min_filter: ImageFilterMode::Linear,
//                             mipmap_filter: ImageFilterMode::Linear,
//                             ..default()
//                         });
//                     }
//                 }
//             }
//         }
//     }
// }
