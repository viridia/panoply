use bevy::prelude::*;

use super::{sync_realms, RealmData, RealmsHandleResource, RealmsLoader};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(RealmsLoader)
            .add_asset::<RealmData>()
            .init_resource::<RealmsHandleResource>()
            .add_systems(Update, sync_realms);
    }
}
