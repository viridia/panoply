use bevy::prelude::*;

/// Margins around the primary camera's viewport, in logical pixels, so that side panels don't
/// overwrite the 3D scene. The editor sets it from its view pane; test plans set it over BRP to
/// shrink the 3D view.
#[derive(Default, Resource, PartialEq, Reflect, Debug)]
#[reflect(Resource)]
pub struct ViewportInset {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// Marks a root UI node drawn over the 3D view (location text, frame time), so a layout that
/// insets the view (the editor's panel) can move it into the view's area.
#[derive(Component, Default, Clone, Copy)]
pub struct ViewOverlay;

/// Marks the UI node that stands for the 3D view in a layout around it (the editor's view pane):
/// pointer input over it, or over what it holds, is meant for the game. Without such a layout, the
/// game takes pointer input that lands on the window itself.
#[derive(Component, Default, Clone, Copy)]
pub struct GameViewNode;
