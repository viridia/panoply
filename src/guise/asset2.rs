use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::{TypePath, TypeUuid},
    utils::{BoxedFuture, HashMap},
};
use serde::{Deserialize, Serialize};

#[derive(TypeUuid, TypePath)]
#[uuid = "29066844-f877-4c2c-9b21-e886de093dfe"]
struct TemplatesAsset {}

#[derive(Serialize, Deserialize, Debug)]
struct TemplatesSerial {
    pub styles: HashMap<String, StyleSetSerial>,
    pub templates: HashMap<String, TemplateSerial>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TemplateSerial {
    // pub base: StylePropsSerial,
    // pub selectors: HashMap<String, StylePropsSerial>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StyleSetSerial(HashMap<String, StyleSerial>);

#[derive(Serialize, Deserialize, Debug)]
struct StyleSerial {
    pub base: StylePropsSerial,
    pub selectors: HashMap<String, StylePropsSerial>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StylePropsSerial {
    vars: HashMap<String, String>,
}

pub struct GuiseTemplatesLoader;

impl AssetLoader for GuiseTemplatesLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let _templates: TemplatesSerial =
                serde_json::from_slice(bytes).expect("unable to decode templates");
            let result: TemplatesAsset = TemplatesAsset {};

            load_context.set_default_asset(LoadedAsset::new(result));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.json"]
    }
}
