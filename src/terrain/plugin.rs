use bevy::prelude::*;

use super::{
    apply_build_parcels, build_parcels,
    material::{create_materials, TerrainMaterial, TerrainMaterials},
    spawn_parcels,
    terrain_shapes::{
        load_terrain_shapes, TerrainShapes, TerrainShapesLoader, TerrainShapesResource,
    },
    ParcelCache,
};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParcelCache::new())
            .add_asset_loader(TerrainShapesLoader)
            .add_asset::<TerrainShapesResource>()
            .init_resource::<TerrainShapes>()
            .init_resource::<TerrainMaterials>()
            .add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            // .insert_resource(TerrainShapes::new())
            .add_systems(Startup, (load_terrain_shapes, create_materials))
            .add_systems(Update, (spawn_parcels, build_parcels, apply_build_parcels));
    }
}
