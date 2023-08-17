use std::sync::Arc;

use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::HashMap;

use super::style::PartialStyle;

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

#[derive(Debug)]
pub enum TemplateNode {
    Element(ElementNode),
    Fragment(ElementNode),
    Text(TextNode),
}

/// Node that represents a 'node' node.
#[derive(Debug, Default)]
pub struct ElementNode {
    /// Inline styles on the node
    /// TODO: Make this RC?
    pub inline_styles: Option<Arc<PartialStyle>>,

    /// Reference to style element
    pub attrs: HashMap<String, String>,

    // ID of this node
    pub id: Option<String>,

    // Attached controller
    pub controller: Option<String>,

    // List of child nodes
    pub children: TemplateNodeList,
    // special attrs
    // each / if / match
}

/// Node that represents a span of text.
#[derive(Debug, Default)]
pub struct TextNode {
    /// Inline styles on the node
    pub inline_styles: Option<Arc<PartialStyle>>,

    // List of child nodes
    pub content: String,
}

#[derive(Debug, Default)]
pub enum TemplateNodeType {
    #[default]
    Node,
    Text,
    Fragment,
    // Show
    // For / Each
    // Switch
    // Fallback
    // Match
    // Call
}
