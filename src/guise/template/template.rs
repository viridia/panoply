use bevy::prelude::Asset;
use bevy::reflect::TypePath;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use super::call::Call;
use super::element::Element;
use super::text::Text;

pub type TemplateNodeList = Vec<Box<TemplateNode>>;

#[derive(Debug, TypePath, Default, Serialize, Deserialize, Asset)]
pub struct TemplateAsset {
    #[serde(default)]
    pub params: HashMap<String, TemplateParam>,
    pub content: Option<Box<TemplateNode>>,
}

/// An instantiable template for a UI node
impl TemplateAsset {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            content: None,
        }
    }
}

/// Defines the types of parameters that can be passed to a template.
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateParam {
    pub r#type: String,
}

impl TemplateParam {
    pub fn new(ty: &str) -> Self {
        Self {
            r#type: ty.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TemplateNode {
    Element(Element),
    Fragment(TemplateNodeList),
    Text(Text),
    Call(Call),
}
