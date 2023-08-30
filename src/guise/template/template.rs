use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use super::element::Element;
use super::text::Text;

pub type NodeList = Vec<Box<Node>>;

#[derive(Debug, TypeUuid, TypePath, Default, Serialize, Deserialize)]
#[uuid = "b2ce477f-e4a4-40cf-b969-916a9dbd799e"]
pub struct Template {
    #[serde(default)]
    pub params: HashMap<String, TemplateParam>,
    pub content: Option<Node>,
}

/// An instantiable template for a UI node
impl Template {
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
pub enum Node {
    Element(Element),
    Fragment(NodeList),
    Text(Text),
}
