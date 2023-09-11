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
    template::{Call, Element, TemplateAsset, TemplateNode, TemplateNodeRef},
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
    // Transform nodes into a form that has resolved handles.
    fn visit_template_node<'a>(
        &self,
        node: &TemplateNodeRef,
        lc: &'a mut LoadContext,
        base: &AssetPath,
    ) -> TemplateNodeRef {
        match node.0.as_ref().as_ref() {
            TemplateNode::Element(elt) => {
                // TODO: Resolve styleset handles.
                TemplateNodeRef::new(TemplateNode::Element(Element {
                    styleset: elt.styleset.clone(),
                    styleset_handles: elt
                        .styleset
                        .iter()
                        .map(|path| lc.load(relative_asset_path(base, &path)))
                        .collect(),
                    inline_style: elt.inline_style.clone(),
                    id: elt.id.clone(),
                    controller: elt.controller.clone(),
                    attrs: elt.attrs.clone(),
                    children: elt
                        .children
                        .iter()
                        .map(|child| self.visit_template_node(child, lc, base))
                        .collect(),
                }))
            }
            TemplateNode::Fragment(frag) => TemplateNodeRef::new(TemplateNode::Fragment(
                frag.iter()
                    .map(|child| self.visit_template_node(child, lc, base))
                    .collect(),
            )),
            TemplateNode::Text(_) => node.clone(),
            TemplateNode::Call(call) => TemplateNodeRef::new(TemplateNode::Call(Call {
                inline_style: call.inline_style.clone(),
                template: call.template.clone(),
                template_handle: lc.load(relative_asset_path(base, &call.template)),
                params: call.params.clone(),
            })),
        }
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
                if let Some(content) = template.content.as_ref() {
                    template.content = Some(self.visit_template_node(content, load_context, &base));
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
