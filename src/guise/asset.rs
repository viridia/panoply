use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::Asset,
    reflect::TypePath,
    utils::{BoxedFuture, HashMap},
};
use futures_lite::AsyncReadExt;
use serde::{Deserialize, Serialize};

use super::{template::TemplateAsset, StyleAsset};

#[derive(TypePath, Asset)]
struct TemplatesAsset {}

#[derive(Serialize, Deserialize, Debug, Asset, TypePath)]
pub struct AssetSerial {
    pub styles: HashMap<String, StyleAsset>,
    pub templates: HashMap<String, TemplateAsset>,
}

pub struct GuiseTemplatesLoader;

impl AssetLoader for GuiseTemplatesLoader {
    type Asset = AssetSerial;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut entries: AssetSerial =
                serde_json::from_slice(&bytes).expect("unable to decode templates");
            entries.styles.drain().for_each(|(key, style)| {
                load_context.add_labeled_asset(format!("styles/{}", key), style);
            });
            entries.templates.drain().for_each(|(key, template)| {
                load_context.add_labeled_asset(format!("templates/{}", key), template);
            });
            Ok(entries)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.json"]
    }
}
