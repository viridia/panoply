use serde_repr::{Deserialize_repr, Serialize_repr};
use std::sync::{Arc, Mutex};

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
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

#[derive(TypeUuid, TypePath)]
#[uuid = "3b62e8db-8f42-485a-adb1-b28ce331bfa7"]
pub struct BiomesAsset(pub Arc<Mutex<BiomesTable>>);

#[derive(Default)]
pub struct BiomesLoader;

impl AssetLoader for BiomesLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let biomes: Vec<BiomeData> =
                serde_json::from_slice(bytes).expect("unable to decode biomes");

            load_context.set_default_asset(LoadedAsset::new(BiomesAsset(Arc::new(Mutex::new(
                BiomesTable { biomes },
            )))));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["biomes.json"]
    }
}

#[derive(Resource, Default)]
pub struct BiomesHandle(pub Handle<BiomesAsset>);

pub fn load_biomes(server: Res<AssetServer>, mut handle: ResMut<BiomesHandle>) {
    handle.0 = server.load("terrain/terrain.biomes.json");
}
