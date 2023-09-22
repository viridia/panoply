use std::sync::Arc;

use bevy::{prelude::Handle, utils::HashMap};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::guise::style::StyleAsset;

use super::{template_expr::TemplateExpr, TemplateAsset};

/// Node that represents an invocation of another template.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Call {
    /// Inline styles on the node
    #[serde(
        rename = "style",
        serialize_with = "serialize_inline_style",
        deserialize_with = "deserialize_inline_style",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub inline_style: Option<Arc<StyleAsset>>,

    // Resource key of template
    pub template: String,

    // Resource key of template
    #[serde(skip)]
    pub template_handle: Handle<TemplateAsset>,

    /// Controller parameters
    #[serde(
        serialize_with = "serialize_params",
        deserialize_with = "deserialize_params",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub params: Option<Arc<HashMap<String, TemplateExpr>>>,
}

fn serialize_inline_style<S: Serializer>(
    st: &Option<Arc<StyleAsset>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    st.as_ref().unwrap().as_ref().serialize(s)
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

fn serialize_params<S: Serializer>(
    st: &Option<Arc<HashMap<String, TemplateExpr>>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    st.as_ref().unwrap().as_ref().serialize(s)
}

fn deserialize_params<'de, D: Deserializer<'de>>(
    de: D,
) -> Result<Option<Arc<HashMap<String, TemplateExpr>>>, D::Error> {
    if let Ok(style) = HashMap::<String, TemplateExpr>::deserialize(de) {
        Ok(Some(Arc::new(style)))
    } else {
        Ok(None)
    }
}
