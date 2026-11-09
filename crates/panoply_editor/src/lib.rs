//! The map editor and its aids, development-only (D13, D37). The app adds [`EditorPlugin`] when
//! its `editor` feature is on. The editor sees only the engine crates: it sets their resources and
//! spawns entities of its own, and the engine does not know it exists.

use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    prelude::*,
};

pub mod debug_views;
pub mod diagnostics;
pub mod goto;
pub mod json;
pub mod mode_selector;
pub mod nav_point_list;
pub mod quit_dialog;
pub mod realm_panel;
pub mod saved_view;
pub mod search_filter;
pub mod shell;
pub mod text_entry;
pub mod toast;
pub mod undo;
pub mod unsaved;
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
            goto::GotoDialogPlugin,
            nav_point_list::NavPointListPlugin,
            mode_selector::ModeSelectorPlugin,
            quit_dialog::QuitDialogPlugin,
            realm_panel::RealmPanelPlugin,
            saved_view::SavedViewPlugin,
            text_entry::TextEntryPlugin,
            toast::ToastPlugin,
            undo::UndoPlugin,
            unsaved::UnsavedAssetsPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()));
    }
}
