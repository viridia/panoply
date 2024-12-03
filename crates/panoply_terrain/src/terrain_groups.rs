use std::sync::{Arc, RwLock};
use thiserror::Error;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct TerrainGroup {
    pub name: String,
    pub visible: bool,

    #[serde(with = "panoply_exemplar::ser::hex_color")]
    pub color: Srgba,
    pub contours: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TerrainGroupsTable(pub Vec<TerrainGroup>);

#[derive(TypePath, Asset, Default)]
pub struct TerrainGroupsAsset(pub Arc<RwLock<TerrainGroupsTable>>);

#[derive(Default)]
pub struct TerrainGroupsLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TerrainGroupsLoaderError {
    #[error("Could not load terrain groups: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for TerrainGroupsLoader {
    type Asset = TerrainGroupsAsset;
    type Settings = ();
    type Error = TerrainGroupsLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let groups: TerrainGroupsTable =
            serde_json::from_slice(&bytes).expect("unable to decode terrain groups");

        Ok(TerrainGroupsAsset(Arc::new(RwLock::new(groups))))
    }

    fn extensions(&self) -> &[&str] {
        &["groups.json"]
    }
}

#[derive(Resource)]
pub struct TerrainGroupsHandle(pub Handle<TerrainGroupsAsset>);

impl FromWorld for TerrainGroupsHandle {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        TerrainGroupsHandle(server.load("terrain/terrain.groups.json"))
    }
}
