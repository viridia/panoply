use bevy::prelude::*;

use super::{
    biome::{load_biomes, BiomesAsset, BiomesHandle, BiomesLoader},
    compute_ground_meshes,
    ground_material::{create_materials, GroundMaterial, TerrainMaterials},
    insert_ground_meshes, spawn_parcels,
    terrain_map::{
        insert_terrain_maps, load_terrain_maps, update_terrain_maps, TerrainMapAsset,
        TerrainMapLoader, TerrainMapsHandleResource,
    },
    terrain_shapes::{
        load_terrain_shapes, TerrainShapesAsset, TerrainShapesHandle, TerrainShapesLoader,
    },
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
            .init_resource::<TerrainMaterials>()
            .init_resource::<TerrainMapsHandleResource>()
            .add_plugins(MaterialPlugin::<GroundMaterial>::default())
            .add_systems(
                Startup,
                (
                    load_biomes,
                    load_terrain_maps,
                    load_terrain_shapes,
                    create_materials,
                ),
            )
            .add_systems(
                Update,
                (
                    spawn_parcels,
                    compute_ground_meshes,
                    insert_ground_meshes,
                    insert_terrain_maps,
                    update_terrain_maps,
                ),
            );
    }
}
