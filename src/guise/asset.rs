use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, LoadContext},
    prelude::Asset,
    reflect::TypePath,
    utils::{BoxedFuture, HashMap},
};
use futures_lite::AsyncReadExt;
use serde::{Deserialize, Serialize};

use super::{
    path::relative_asset_path,
    template::{TemplateAsset, TemplateNode},
    StyleAsset,
};

#[derive(TypePath, Asset)]
struct TemplatesAsset {}

#[derive(Serialize, Deserialize, Debug, Asset, TypePath)]
pub struct AssetSerial {
    pub styles: HashMap<String, StyleAsset>,
    pub templates: HashMap<String, TemplateAsset>,
}

pub struct GuiseTemplatesLoader;

impl GuiseTemplatesLoader {
    fn visit_node<'a>(&self, node: &mut TemplateNode, lc: &'a mut LoadContext, base: &AssetPath) {
        match node {
            TemplateNode::Element(elt) => elt
                .children
                .iter_mut()
                .for_each(|child| self.visit_node(child, lc, base)),
            TemplateNode::Fragment(_) => todo!(),
            TemplateNode::Text(_) => {}
            TemplateNode::Call(call) => {
                call.template_handle = lc.load(relative_asset_path(base, &call.template))
            }
        };
    }
}

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
            entries.templates.drain().for_each(|(key, mut template)| {
                let label = format!("templates/{}", key);
                // TODO: Lots of string copying here.
                let base = AssetPath::new(
                    load_context.path().to_path_buf().clone(),
                    Some(label.clone()),
                );
                load_context.begin_labeled_asset();
                if let Some(content) = template.content.as_mut() {
                    self.visit_node(content, load_context, &base);
                }
                load_context.add_labeled_asset(label, template);
            });
            Ok(entries)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.json"]
    }
}
