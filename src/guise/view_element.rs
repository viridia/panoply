use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use super::{
    computed::{ComputedStyle, UpdateComputedStyle},
    Expr, RenderOutput, Renderable,
};

/// Component that defines a ui element, and which can differentially update when the
/// template asset changes.
#[derive(Component)]
pub struct ViewElement {
    /// Template node used to generate this element
    pub(crate) template: Arc<dyn Renderable>,

    /// Element id
    pub id: Option<String>,

    /// Cached handles for stylesets.
    pub style: Vec<Expr>,

    /// ID of presenter component associated with this element.
    pub presenter: Option<String>,

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

pub fn update_view_element_styles(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut ViewElement,
            // One<&dyn Controller>,
            Option<&Parent>,
        ),
        Changed<ViewElement>,
    >,
) {
    for (entity, view, _parent) in query.iter_mut() {
        let mut computed = ComputedStyle::new();
        for style_expr in view.style.iter() {
            match style_expr {
                Expr::Style(style) => {
                    style.apply_to(&mut computed);
                }
                _ => {
                    panic!("Expression is not a style");
                }
            }
        }

        commands.add(UpdateComputedStyle { entity, computed });
    }
}
