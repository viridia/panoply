use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::{TypePath, TypeUuid},
    utils::{BoxedFuture, HashMap},
};
use serde::{Deserialize, Serialize};

use super::{template::Template, StyleAsset};

#[derive(TypeUuid, TypePath)]
#[uuid = "29066844-f877-4c2c-9b21-e886de093dfe"]
struct TemplatesAsset {}

#[derive(Serialize, Deserialize, Debug)]
struct AssetSerial {
    pub styles: HashMap<String, StyleAsset>,
    pub templates: HashMap<String, Template>,
}

pub struct GuiseTemplatesLoader;

impl AssetLoader for GuiseTemplatesLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let mut entries: AssetSerial =
                serde_json::from_slice(bytes).expect("unable to decode templates");
            entries.styles.drain().for_each(|(key, style)| {
                load_context
                    .set_labeled_asset(format!("styles/{}", key).as_str(), LoadedAsset::new(style));
            });
            entries.templates.drain().for_each(|(key, style)| {
                load_context.set_labeled_asset(
                    format!("templates/{}", key).as_str(),
                    LoadedAsset::new(style),
                );
            });
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.json"]
    }
}
