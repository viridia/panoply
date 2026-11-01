//! The map editor and its aids, development-only (D13, D37). The app adds [`EditorPlugin`] when
//! its `editor` feature is on. The editor sees only the engine crates: it sets their resources and
//! spawns entities of its own, and the engine does not know it exists.

use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    prelude::*,
};

pub mod debug_views;
pub mod diagnostics;
pub mod shell;
pub mod zoom;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FeathersPlugins,
            shell::EditorShellPlugin,
            debug_views::DebugViewsPlugin,
            diagnostics::EditorDiagnosticsPlugin,
            zoom::EditorZoomPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()));
    }
}
