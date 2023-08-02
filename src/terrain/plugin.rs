use bevy::{
    prelude::*,
    render::{
        render_resource::{AddressMode, FilterMode, SamplerDescriptor},
        texture::ImageSampler,
    },
};

use super::{
    biome::{load_biomes, BiomesAsset, BiomesHandle, BiomesLoader},
    gen_ground_meshes,
    ground_material::GroundMaterial,
    insert_ground_meshes, spawn_parcels,
    terrain_map::{
        insert_terrain_maps, load_terrain_maps, update_ground_material, update_terrain_maps,
        TerrainMapAsset, TerrainMapLoader, TerrainMapsHandleResource,
    },
    terrain_shapes::{
        load_terrain_shapes, TerrainShapesAsset, TerrainShapesHandle, TerrainShapesLoader,
    },
    water_material::{create_water_material, WaterMaterial, WaterMaterialResource},
    water_mesh::{gen_water_meshes, insert_water_meshes},
    ParcelCache,
};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParcelCache::new())
            .add_asset_loader(TerrainShapesLoader)
            .add_asset_loader(TerrainMapLoader)
            .add_asset_loader(BiomesLoader)
            .add_asset::<TerrainShapesAsset>()
            .add_asset::<TerrainMapAsset>()
            .add_asset::<BiomesAsset>()
            .init_resource::<BiomesHandle>()
            .init_resource::<TerrainShapesHandle>()
            .init_resource::<TerrainMapsHandleResource>()
            .init_resource::<WaterMaterialResource>()
            .add_plugins((
                MaterialPlugin::<GroundMaterial>::default(),
                MaterialPlugin::<WaterMaterial>::default(),
            ))
            .add_systems(
                Startup,
                (
                    load_biomes,
                    load_terrain_maps,
                    load_terrain_shapes,
                    create_water_material,
                ),
            )
            .add_systems(
                Update,
                (
                    spawn_parcels,
                    gen_ground_meshes,
                    gen_water_meshes,
                    insert_ground_meshes,
                    insert_water_meshes,
                    insert_terrain_maps,
                    update_terrain_maps,
                    update_ground_material,
                    config_textures_modes,
                ),
            );
    }
}

pub fn config_textures_modes(
    server: Res<AssetServer>,
    mut assets: ResMut<Assets<Image>>,
    mut ev_image: EventReader<AssetEvent<Image>>,
) {
    for ev in ev_image.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    let path = asset_path.path();
                    if path.parent().expect("path").to_str().expect("path") == "textures" {
                        if let Some(image) = assets.get_mut(handle) {
                            image.sampler_descriptor =
                                ImageSampler::Descriptor(SamplerDescriptor {
                                    label: Some("Terrain textures"),
                                    address_mode_u: AddressMode::Repeat,
                                    address_mode_v: AddressMode::ClampToEdge,
                                    address_mode_w: AddressMode::ClampToEdge,
                                    mag_filter: FilterMode::Linear,
                                    min_filter: FilterMode::Linear,
                                    mipmap_filter: FilterMode::Linear,
                                    ..default()
                                });
                        }
                    }
                }
            }

            _ => {}
        }
    }
}
