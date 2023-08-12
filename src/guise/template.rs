use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

pub type UiNodeList = Vec<Box<UiNode>>;

#[derive(Debug, TypeUuid, TypePath, Serialize, Deserialize, Default)]
#[uuid = "b2ce477f-e4a4-40cf-b969-916a9dbd799e"]
pub struct Template {
    pub params: HashMap<String, ParamType>,
    pub children: UiNodeList,
}

impl Template {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ParamType {
    Bool,
    I32,
    U32,
    F32,
    String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UiNode {
    pub tag: UiNodeType,
    // attrs
    // style
    pub controller: Option<String>,
    pub children: UiNodeList,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UiNodeType {
    Node,
    Flex,
    Grid,
    // Fragment
    // Show
    // For / Each
    // Switch
    // Fallback
    // Match
    // Call
}
