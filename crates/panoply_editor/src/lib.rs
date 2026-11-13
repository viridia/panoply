//! The map editor and its aids, development-only (D13, D37). The app adds [`EditorPlugins`] when
//! its `editor` feature is on. The editor sees only the engine crates: it sets their resources and
//! spawns entities of its own, and the engine does not know it exists.

use bevy::{
    app::PluginGroupBuilder,
    feathers::{
        dark_theme::create_dark_theme,
        palette,
        theme::{SemanticToken, SurfaceLevel, ThemeProps, UiTheme},
        tokens, FeathersPlugins,
    },
    prelude::*,
    ui::FixedNode,
    ui_widgets::popover::Popover,
};

pub mod aspect_chooser;
pub mod chooser;
pub mod debug_views;
pub mod diagnostics;
pub mod exemplar_list;
pub mod goto;
pub mod json;
pub mod map_size;
pub mod mode_overlays;
pub mod mode_selector;
pub mod model_picker;
pub mod nav_point_list;
pub mod new_realm;
pub mod parcel_selection;
pub mod properties;
pub mod quit_dialog;
pub mod realm_panel;
pub mod saved_view;
pub mod scenery_panel;
pub mod scenery_properties;
pub mod search_filter;
pub mod shell;
pub mod terrain_cursor;
pub mod terrain_map_edit;
pub mod terrain_panel;
pub mod text_commit;
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
            .add(FixedPopoversPlugin)
            .add(shell::EditorShellPlugin)
            .add(debug_views::DebugViewsPlugin)
            .add(diagnostics::EditorDiagnosticsPlugin)
            .add(zoom::EditorZoomPlugin)
            .add(goto::GotoDialogPlugin)
            .add(panoply_overlays::OverlaysPlugin)
            .add(mode_overlays::ModeOverlaysPlugin)
            .add(chooser::ChooserPlugin)
            .add(model_picker::ModelPickerPlugin)
            .add(nav_point_list::NavPointListPlugin)
            .add(new_realm::NewRealmPlugin)
            .add(map_size::MapSizePlugin)
            .add(parcel_selection::ParcelSelectionPlugin)
            .add(mode_selector::ModeSelectorPlugin)
            .add(quit_dialog::QuitDialogPlugin)
            .add(realm_panel::RealmPanelPlugin)
            .add(terrain_panel::TerrainPanelPlugin)
            .add(scenery_panel::SceneryPanelPlugin)
            .add(scenery_properties::SceneryPropertiesPlugin)
            .add(exemplar_list::ExemplarListPlugin)
            .add(terrain_cursor::TerrainCursorPlugin)
            .add(saved_view::SavedViewPlugin)
            .add(text_commit::TextCommitPlugin)
            .add(text_entry::TextEntryPlugin)
            .add(toast::ToastPlugin)
            .add(undo::UndoPlugin)
            .add(unsaved::UnsavedAssetsPlugin)
    }
}

/// Feathers' dark theme, which its widgets read their colors from, with a fix of our own.
struct EditorThemePlugin;

impl Plugin for EditorThemePlugin {
    fn build(&self, app: &mut App) {
        let mut theme = create_dark_theme();
        raise_input_labels(&mut theme);
        app.insert_resource(UiTheme(theme));
    }
}

/// Every popover (a menu's or a select's popup) laid out as a root of its own, a `FixedNode`,
/// rather than inside its parent's layout: an absolutely positioned popup still counts toward its
/// containing block's scrollable overflow, so one below the last select in a scrolling panel
/// stretched the panel's scroll range, open or closed (I44). It keeps its place in the hierarchy,
/// so picking, despawning and inherited theme and font still follow its parent; `Popover` places
/// it from its parent's on-screen rect, which a `FixedNode` does not change.
// TODO(I44): drop once `Popover` requires `FixedNode` upstream.
struct FixedPopoversPlugin;

impl Plugin for FixedPopoversPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Popover, FixedNode>();
    }
}

/// A number input's label (the X, Y or Z beside an axis's sigil) shares the solid fill, which a
/// subpane body (`SurfaceLevel::Higher`) leaves at the gray it raises the input itself to, so the
/// label vanished into its input (I37). The label gets a fill of its own, a step lighter there,
/// as the theme lightens `GRAY_3` where it has no lighter gray. Buttons keep the solid fill.
// TODO(I37): drop once feathers' theme separates the two upstream.
fn raise_input_labels(theme: &mut ThemeProps) {
    const LABEL_FILL: SemanticToken = SemanticToken::new_static("panoply.fill.label");
    theme
        .token_assignments
        .insert(tokens::TEXT_INPUT_LABEL_BG, LABEL_FILL);
    theme.semantic_base.insert(LABEL_FILL, palette::GRAY_3);
    theme
        .semantic_overrides
        .entry(SurfaceLevel::Higher)
        .or_default()
        .insert(LABEL_FILL, palette::GRAY_3.lighter(0.05));
}
