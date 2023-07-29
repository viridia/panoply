use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct FloraTableEntry {
    proto: Option<String>,
    probability: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct BiomeData {
    pub surface: String,
    pub trees: Vec<FloraTableEntry>,
    pub shrubs: Vec<FloraTableEntry>,
    pub herbs: Vec<FloraTableEntry>,
}

pub struct _Biome {}

#[derive(TypeUuid, TypePath)]
#[uuid = "3b62e8db-8f42-485a-adb1-b28ce331bfa7"]
pub struct BiomesAsset(pub Arc<Mutex<HashMap<String, BiomeData>>>);

#[derive(Default)]
pub struct BiomesLoader;

impl AssetLoader for BiomesLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let res: HashMap<String, BiomeData> =
                serde_json::from_slice(bytes).expect("unable to decode biomes");

            load_context
                .set_default_asset(LoadedAsset::new(BiomesAsset(Arc::new(Mutex::new(res)))));
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
