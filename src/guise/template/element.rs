use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::guise::style::Style;

use super::template::NodeList;

/// Node that represents a template element.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Element {
    /// Reference to named style.
    #[serde(rename = "styleset")]
    pub style: Option<String>,

    /// Inline styles on the node
    #[serde(rename = "style")]
    pub inline_style: Option<Style>,

    // ID of this node
    pub id: Option<String>,

    // Attached controller
    pub controller: Option<String>,

    /// Controller parameters
    #[serde(flatten)]
    pub attrs: HashMap<String, String>,

    // List of child nodes
    #[serde(default)]
    pub children: NodeList,
    // special attrs
    // each / if / match
}
