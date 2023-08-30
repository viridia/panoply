use serde::{Deserialize, Serialize};

use crate::guise::style::StyleAsset;

/// Node that represents a span of text.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Text {
    /// Reference to named style.
    pub style: Option<String>,

    /// Inline styles on the node
    pub inline_styles: Option<StyleAsset>,

    // List of child nodes
    pub content: String,
}
