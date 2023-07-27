use bevy::prelude::*;

use super::{load_realms, sync_realms, RealmData, RealmsHandleResource, RealmsLoader};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(RealmsLoader)
            .add_asset::<RealmData>()
            .init_resource::<RealmsHandleResource>()
            // .init_resource::<TerrainMaterials>()
            // .add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(Startup, load_realms)
            .add_systems(Update, sync_realms);
    }
}
