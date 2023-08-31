use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext, LoadedAsset},
    reflect::{TypePath, TypeUuid},
    utils::{BoxedFuture, HashMap},
};
use serde::{Deserialize, Serialize};

use super::{
    path::relative_asset_path,
    template::{TemplateAsset, TemplateNode},
    StyleAsset,
};

#[derive(TypeUuid, TypePath)]
#[uuid = "29066844-f877-4c2c-9b21-e886de093dfe"]
struct TemplatesAsset {}

#[derive(Serialize, Deserialize, Debug)]
struct AssetSerial {
    pub styles: HashMap<String, StyleAsset>,
    pub templates: HashMap<String, TemplateAsset>,
}

pub struct GuiseTemplatesLoader;

impl GuiseTemplatesLoader {
    fn visit_template<'a>(&self, template: &mut TemplateAsset, lc: &LoadContext<'a>) {
        if let Some(content) = template.content.as_mut().as_mut() {
            self.visit_node(content, lc)
        }
    }

    fn visit_node<'a>(&self, node: &mut TemplateNode, lc: &LoadContext<'a>) {
        match node {
            TemplateNode::Element(element) => {
                if element.styleset.len() > 0 {
                    let base = AssetPath::from(lc.path());
                    element.styleset_handles.reserve(element.styleset.len());
                    for style_path in element.styleset.iter() {
                        let path = relative_asset_path(&base, style_path);
                        element.styleset_handles.push(lc.get_handle(path));
                    }
                }
                for child in element.children.iter_mut() {
                    self.visit_node(child, lc)
                }
            }

            _ => (),
        }
    }
}

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
            entries.templates.drain().for_each(|(key, mut template)| {
                self.visit_template(&mut template, &load_context);
                let asset = LoadedAsset::new(template);
                load_context.set_labeled_asset(format!("templates/{}", key).as_str(), asset);
            });
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.json"]
    }
}
