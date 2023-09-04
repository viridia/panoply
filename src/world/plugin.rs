use bevy::prelude::*;

use super::{
    sync_realms, RealmData, RealmsHandleResource, RealmsLoader, WorldLocationsAsset,
    WorldLocationsLoader, WorldLocationsResource,
};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(RealmsLoader)
            .register_asset_loader(WorldLocationsLoader)
            .init_asset::<RealmData>()
            .init_asset::<WorldLocationsAsset>()
            .init_resource::<RealmsHandleResource>()
            .init_resource::<WorldLocationsResource>()
            .add_systems(Update, sync_realms);
    }
}
