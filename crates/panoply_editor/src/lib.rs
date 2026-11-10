//! The map editor and its aids, development-only (D13, D37). The app adds [`EditorPlugins`] when
//! its `editor` feature is on. The editor sees only the engine crates: it sets their resources and
//! spawns entities of its own, and the engine does not know it exists.

use bevy::{
    app::PluginGroupBuilder,
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    prelude::*,
};

pub mod chooser;
pub mod debug_views;
pub mod diagnostics;
pub mod goto;
pub mod json;
pub mod map_size;
pub mod mode_overlays;
pub mod mode_selector;
pub mod nav_point_list;
pub mod new_realm;
pub mod parcel_selection;
pub mod quit_dialog;
pub mod realm_panel;
pub mod saved_view;
pub mod search_filter;
pub mod shell;
pub mod terrain_map_edit;
pub mod terrain_panel;
pub mod text_entry;
pub mod toast;
pub mod undo;
pub mod unsaved;
pub mod zoom;

pub struct EditorPlugins;

impl PluginGroup for EditorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(FeathersPlugins)
            .add(EditorThemePlugin)
            .add(shell::EditorShellPlugin)
            .add(debug_views::DebugViewsPlugin)
            .add(diagnostics::EditorDiagnosticsPlugin)
            .add(zoom::EditorZoomPlugin)
            .add(goto::GotoDialogPlugin)
            .add(panoply_overlays::OverlaysPlugin)
            .add(mode_overlays::ModeOverlaysPlugin)
            .add(chooser::ChooserPlugin)
            .add(nav_point_list::NavPointListPlugin)
            .add(new_realm::NewRealmPlugin)
            .add(map_size::MapSizePlugin)
            .add(parcel_selection::ParcelSelectionPlugin)
            .add(mode_selector::ModeSelectorPlugin)
            .add(quit_dialog::QuitDialogPlugin)
            .add(realm_panel::RealmPanelPlugin)
            .add(terrain_panel::TerrainPanelPlugin)
            .add(saved_view::SavedViewPlugin)
            .add(text_entry::TextEntryPlugin)
            .add(toast::ToastPlugin)
            .add(undo::UndoPlugin)
            .add(unsaved::UnsavedAssetsPlugin)
    }
}

/// Feathers' dark theme, which its widgets read their colors from.
struct EditorThemePlugin;

impl Plugin for EditorThemePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiTheme(create_dark_theme()));
    }
}
