use futures_lite::AsyncReadExt;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::sync::{Arc, Mutex};
use thiserror::Error;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};

use crate::random::Choice;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Default, Copy, Clone)]
#[repr(u8)]
pub enum BiomeSurfaceType {
    #[default]
    Rock = 0,
    Dirt = 1,
    Grass = 2,
    Moss = 3,
    Taiga = 4,
    Sand = 5,
    Tundra = 6,
    Snow = 7,
    Chaparral = 8,
}

#[derive(Default, Serialize, Deserialize)]
pub struct FloraTableEntry {
    pub proto: Option<String>,
    probability: f32,
}

impl Choice for FloraTableEntry {
    fn probability(&self) -> f32 {
        self.probability
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct BiomeData {
    pub name: String,
    pub surface: BiomeSurfaceType,
    pub trees: Vec<FloraTableEntry>,
    pub shrubs: Vec<FloraTableEntry>,
    pub herbs: Vec<FloraTableEntry>,
}

pub struct BiomesTable {
    pub biomes: Vec<BiomeData>,
}

#[derive(TypePath, Asset)]
pub struct BiomesAsset(pub Arc<Mutex<BiomesTable>>);

#[derive(Default)]
pub struct BiomesLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum BiomesLoaderError {
    #[error("Could not load biome: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for BiomesLoader {
    type Asset = BiomesAsset;
    type Settings = ();
    type Error = BiomesLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let biomes: Vec<BiomeData> =
            serde_json::from_slice(&bytes).expect("unable to decode biomes");

        Ok(BiomesAsset(Arc::new(Mutex::new(BiomesTable { biomes }))))
    }

    fn extensions(&self) -> &[&str] {
        &["biomes.json"]
    }
}

#[derive(Resource)]
pub struct BiomesHandle(pub Handle<BiomesAsset>);

impl FromWorld for BiomesHandle {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        BiomesHandle(server.load("terrain/terrain.biomes.json"))
    }
}
