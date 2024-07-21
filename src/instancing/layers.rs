use bevy::prelude::Component;

/// A component that indicates that we want to propagate render layers to all descendants.
#[derive(Debug, Clone, Copy, Component)]
pub struct PropagateRenderLayers;
