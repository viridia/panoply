use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::utils::BoxedFuture;
use futures_lite::AsyncReadExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(TypePath, Asset)]
pub struct WorldLocationsAsset(pub Vec<WorldLocation>);

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum WorldLocationsLoaderError {
    #[error("Could not load locations: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct WorldLocationsLoader;

impl AssetLoader for WorldLocationsLoader {
    type Asset = WorldLocationsAsset;
    type Error = WorldLocationsLoaderError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let locations: Vec<WorldLocation> =
                serde_json::from_slice(&bytes).expect("unable to decode locations");

            Ok(WorldLocationsAsset(locations))
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
