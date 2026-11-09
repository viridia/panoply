//! The Go To dialog: G opens it, listing the nav points to travel to.

use bevy::prelude::*;
use bevy_action_map::prelude::*;
use panoply_core::{NavPointsAsset, NavPointsHandleResource};

use crate::{
    chooser::{chooser_dialog, ItemChosen},
    nav_point_list::{nav_point_names, Travel},
    shell::EditorShellControl,
};

/// Open the Go To dialog.
#[derive(InputAction)]
#[action(path = "editor.goto", output = bool, intent = Button)]
pub struct OpenGotoDialog;

/// The dialog's root.
#[derive(Component, Default, Clone)]
struct GotoDialog;

pub struct GotoDialogPlugin;

impl Plugin for GotoDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, open_dialog);
    }
}

/// While the dialog is open it holds the keys (`widget_keys`), so G there types in the filter.
fn open_dialog(
    input: ContextActions<EditorShellControl>,
    dialogs: Query<(), With<GotoDialog>>,
    handle: Res<NavPointsHandleResource>,
    assets: Res<Assets<NavPointsAsset>>,
    mut commands: Commands,
) {
    if input.fired::<OpenGotoDialog>() && dialogs.is_empty() {
        let names = nav_point_names(&handle, &assets);
        commands
            .spawn_scene(chooser_dialog("Go To", "Go", names))
            .insert((Name::new("GotoDialog"), GotoDialog))
            .observe(|chosen: On<ItemChosen>, mut travel: Travel| travel.to(&chosen.name));
    }
}
