use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use super::{renderable::Renderable, Expr, RenderOutput};

/// Component that defines a ui element, and which can differentially update when the
/// template asset changes.
#[derive(Component)]
pub struct ViewElement {
    /// Template node used to generate this element
    pub(crate) template: Arc<dyn Renderable + Sync + Send>,

    /// Element id
    pub id: Option<String>,

    /// Cached handles for stylesets.
    pub style: Vec<Expr>,

    /// ID of controller component associated with this element.
    pub controller: Option<String>,

    // Class names used for style selectors.
    pub classes: Vec<String>,

    /// Generated list of entities
    pub(crate) children: Vec<RenderOutput>,

    // Template properties
    pub(crate) props: Arc<HashMap<String, Expr>>,
    // Other possible props:
    // memoized - whether this node should be re-evaluated when parent changes.
    // template parameters
    // context vars, inherited context vars.
    // 'modified' flag. That should probably be a separate component.
    // Idea: what about having the view nodes be separate entities from the ui nodes?
}
