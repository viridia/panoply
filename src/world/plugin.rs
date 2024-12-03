use bevy::prelude::*;
use panoply_core::PanoplyCorePlugin;

use super::{WorldLocationsAsset, WorldLocationsLoader, WorldLocationsResource};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanoplyCorePlugin)
            .register_asset_loader(WorldLocationsLoader)
            .init_asset::<WorldLocationsAsset>()
            .init_resource::<WorldLocationsResource>();
    }
}
