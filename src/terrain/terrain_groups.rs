use futures_lite::AsyncReadExt;
use std::sync::{Arc, RwLock};
use thiserror::Error;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};

use crate::reflect_types::HexColor;

#[derive(Default, Serialize, Deserialize)]
pub struct TerrainGroup {
    pub name: String,
    pub visible: bool,
    pub color: HexColor,
    pub contours: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TerrainGroupsTable(pub Vec<TerrainGroup>);

#[derive(TypePath, Asset)]
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

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
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
