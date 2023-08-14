use std::sync::Arc;

use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::HashMap;

use super::partial_style::PartialStyle;

pub type TemplateNodeList = Vec<Box<TemplateNode>>;

#[derive(Debug, TypeUuid, TypePath, Default)]
#[uuid = "b2ce477f-e4a4-40cf-b969-916a9dbd799e"]
pub struct Template {
    pub params: HashMap<String, TemplateParam>,
    pub children: TemplateNodeList,
}

/// An instantiable template for a UI node
impl Template {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            children: Vec::new(),
        }
    }
}

/// Defines the types of parameters that can be passed to a template.
#[derive(Debug)]
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

#[derive(Debug, Default)]
pub struct TemplateNode {
    /// Type of node
    pub tag: TemplateNodeType,

    /// Inline styles on the node
    /// TODO: Make this RC?
    pub inline_styles: Option<Arc<PartialStyle>>,

    /// Reference to style element
    pub attrs: HashMap<String, String>,

    pub controller: Option<String>,
    pub children: TemplateNodeList,
    // special attrs
    // each / if / match
}

#[derive(Debug, Default)]
pub enum TemplateNodeType {
    #[default]
    Node,
    Flex,
    Grid,
    Fragment,
    // Show
    // For / Each
    // Switch
    // Fallback
    // Match
    // Call
}
