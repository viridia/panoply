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
use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};
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
                    // config_textures_modes,
                    spawn_flora_model_instances,
                ),
            );
    }
}

pub fn config_textures_modes(
    server: Res<AssetServer>,
    mut assets: ResMut<Assets<Image>>,
    mut ev_image: EventReader<AssetEvent<Image>>,
) {
    for ev in ev_image.read() {
        if let AssetEvent::Added { id } = ev {
            if let Some(asset_path) = server.get_path(*id) {
                let path = asset_path.path();
                if path.parent().expect("path").to_str().expect("path") == "textures" {
                    if let Some(image) = assets.get_mut(*id) {
                        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                            label: Some("Terrain textures".to_string()),
                            address_mode_u: ImageAddressMode::Repeat,
                            address_mode_v: ImageAddressMode::ClampToEdge,
                            address_mode_w: ImageAddressMode::ClampToEdge,
                            mag_filter: ImageFilterMode::Linear,
                            min_filter: ImageFilterMode::Linear,
                            mipmap_filter: ImageFilterMode::Linear,
                            ..default()
                        });
                    }
                }
            }
        }
    }
}
