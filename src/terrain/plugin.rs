use bevy::prelude::*;

use super::{
    compute_ground_meshes, insert_ground_meshes,
    material::{create_materials, TerrainMaterial, TerrainMaterials},
    spawn_parcels,
    terrain_map::{load_terrain_maps, TerrainMapsHandleResource},
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
            .add_asset::<TerrainShapesAsset>()
            // .init_resource::<TerrainShapesHandle>()
            .init_resource::<TerrainShapesHandle>()
            .init_resource::<TerrainMaterials>()
            .init_resource::<TerrainMapsHandleResource>()
            .add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                Startup,
                (load_terrain_maps, load_terrain_shapes, create_materials),
            )
            .add_systems(
                Update,
                (spawn_parcels, compute_ground_meshes, insert_ground_meshes),
            );
    }
}
