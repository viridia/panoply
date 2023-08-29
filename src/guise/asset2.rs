use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::{TypePath, TypeUuid},
    utils::{BoxedFuture, HashMap},
};
use serde::{Deserialize, Serialize};

use super::style::StyleCatalog;

#[derive(TypeUuid, TypePath)]
#[uuid = "29066844-f877-4c2c-9b21-e886de093dfe"]
struct TemplatesAsset {}

#[derive(Serialize, Deserialize, Debug)]
struct TemplatesSerial<'a> {
    pub styles: HashMap<String, StyleCatalog<'a>>,
    pub templates: HashMap<String, TemplateSerial>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TemplateSerial {
    // pub base: StylePropsSerial,
    // pub selectors: HashMap<String, StylePropsSerial>,
}

pub struct GuiseTemplatesLoader;

impl AssetLoader for GuiseTemplatesLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let entries: TemplatesSerial =
                serde_json::from_slice(bytes).expect("unable to decode templates");
            entries.styles.iter().for_each(|(key, style)| {
                // load_context.set_labeled_asset(key, LoadedAsset::new(*style));
            });

            // load_context.set_default_asset(LoadedAsset::new(result));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.json"]
    }
}
