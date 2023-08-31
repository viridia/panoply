use std::sync::Arc;

use bevy::{prelude::Handle, utils::HashMap};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::guise::style::StyleAsset;

use super::template::TemplateNodeList;

/// Node that represents a template element.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Element {
    /// Reference to named style.
    #[serde(rename = "styleset", default)]
    pub styleset: Vec<String>,

    #[serde(skip)]
    pub styleset_handles: Vec<Handle<StyleAsset>>,

    /// Inline styles on the node
    #[serde(
        rename = "style",
        serialize_with = "serialize_inline_style",
        deserialize_with = "deserialize_inline_style",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub inline_style: Option<Arc<StyleAsset>>,

    // ID of this node
    pub id: Option<String>,

    // Attached controller
    pub controller: Option<String>,

    /// Controller parameters
    #[serde(flatten)]
    pub attrs: HashMap<String, String>,

    // List of child nodes
    #[serde(default)]
    pub children: TemplateNodeList,
    // special attrs
    // each / if / match
}

fn serialize_inline_style<S: Serializer>(
    st: &Option<Arc<StyleAsset>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let style = st.as_ref().unwrap().as_ref();
    style.serialize(s)
}

fn deserialize_inline_style<'de, D: Deserializer<'de>>(
    de: D,
) -> Result<Option<Arc<StyleAsset>>, D::Error> {
    if let Ok(style) = StyleAsset::deserialize(de) {
        Ok(Some(Arc::new(style)))
    } else {
        Ok(None)
    }
}
