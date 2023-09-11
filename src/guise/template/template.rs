use std::ops::Deref;
use std::sync::Arc;

use bevy::prelude::Asset;
use bevy::reflect::TypePath;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use super::call::Call;
use super::element::Element;
use super::text::Text;

pub type TemplateNodeList = Vec<TemplateNodeRef>;

#[derive(Debug, TypePath, Default, Serialize, Deserialize, Asset)]
pub struct TemplateAsset {
    #[serde(default)]
    pub params: HashMap<String, TemplateParam>,
    pub content: Option<TemplateNodeRef>,
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
    // Cond
    // Each
}

#[derive(Debug, Deserialize, Clone)]
#[serde(from = "TemplateNode")]
pub struct TemplateNodeRef(pub Arc<Box<TemplateNode>>);

impl Deref for TemplateNodeRef {
    type Target = TemplateNode;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl AsRef<TemplateNode> for TemplateNodeRef {
    fn as_ref(&self) -> &TemplateNode {
        self.0.as_ref()
    }
}

impl Serialize for TemplateNodeRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

impl From<TemplateNode> for TemplateNodeRef {
    fn from(value: TemplateNode) -> Self {
        TemplateNodeRef(Arc::new(Box::new(value)))
    }
}
