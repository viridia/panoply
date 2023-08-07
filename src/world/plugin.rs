use bevy::prelude::*;

use super::{
    sync_realms, RealmData, RealmsHandleResource, RealmsLoader, WorldLocationsAsset,
    WorldLocationsLoader, WorldLocationsResource,
};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(RealmsLoader)
            .add_asset_loader(WorldLocationsLoader)
            .add_asset::<RealmData>()
            .add_asset::<WorldLocationsAsset>()
            .init_resource::<RealmsHandleResource>()
            .init_resource::<WorldLocationsResource>()
            .add_systems(Update, sync_realms);
    }
}
