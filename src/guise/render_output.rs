use bevy::prelude::*;

/// Output of a rendered node, which may be a single node or a fragment (multiple nodes).
/// This gets flattened before attaching to the parent UiNode.
#[derive(Debug, PartialEq, Clone)]
pub enum RenderOutput {
    // Means that nothing was rendered. This can represent either an initial state
    // before the first render, or a conditional render operation.
    Empty,

    // Template rendered a single node
    Node(Entity),

    // Template rendered a fragment or a list of nodes.
    Fragment(Box<[RenderOutput]>),
}

impl RenderOutput {
    /// Returns the number of actual entities in the template output.
    pub fn count(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Node(_) => 1,
            Self::Fragment(nodes) => nodes.iter().map(|node| node.count()).sum(),
        }
    }

    /// Flattens the list of entities into a vector.
    pub fn flatten(&self, out: &mut Vec<Entity>) {
        match self {
            Self::Empty => {}
            Self::Node(entity) => out.push(*entity),
            Self::Fragment(nodes) => nodes.iter().for_each(|node| node.flatten(out)),
        }
    }

    /// Despawn all entities held.
    pub(crate) fn despawn_recursive(&self, commands: &mut Commands) {
        match self {
            Self::Empty => {}
            Self::Node(entity) => commands.entity(*entity).despawn_recursive(),
            Self::Fragment(nodes) => nodes
                .iter()
                .for_each(|node| node.despawn_recursive(commands)),
        }
    }
}

impl Default for RenderOutput {
    fn default() -> Self {
        Self::Empty
    }
}
