use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::BoxedFuture;
use serde::{Deserialize, Serialize};

/// Represents a map location or nav point that can be teleported to.
#[derive(Default, Serialize, Deserialize)]
pub struct WorldLocation {
    /// Realm name containing this location
    pub realm: String,

    /// Position to teleport to.
    pub pos: Vec3,

    /// Id of this location
    pub name: String,

    /// Human-readable name of this location
    pub caption: Option<String>,

    /// Icon that appears on the map. If None, then this is not visible on the map.
    /// 'village' | 'town' | 'keep' | 'cave' | 'ruin' | 'camp';
    pub icon: Option<String>,

    /// Location of icon on map, defaults to 'pos' if not present.
    pub map_pos: Option<Vec3>,
}

#[derive(TypeUuid, TypePath)]
#[uuid = "fcd3a8fa-ab24-4938-b047-e7c71571a06b"]
pub struct WorldLocationsAsset(pub Vec<WorldLocation>);

#[derive(Default)]
pub struct WorldLocationsLoader;

impl AssetLoader for WorldLocationsLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let locations: Vec<WorldLocation> =
                serde_json::from_slice(bytes).expect("unable to decode biomes");

            load_context.set_default_asset(LoadedAsset::new(WorldLocationsAsset(locations)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["locations.json"]
    }
}

#[derive(Resource)]
pub struct WorldLocationsResource(pub Handle<WorldLocationsAsset>);

impl FromWorld for WorldLocationsResource {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        WorldLocationsResource(server.load("world.locations.json"))
    }
}
