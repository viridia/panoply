use bevy::prelude::*;

use super::instance::create_mesh_instances;

pub struct InstancedModelsPlugin;

impl Plugin for InstancedModelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, create_mesh_instances);
        // .init_asset_loader::<ModelLoader>();
    }
}
