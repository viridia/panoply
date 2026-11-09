//! While assets are unsaved, quitting asks first: Save, Discard or Cancel. A save that fails leaves
//! the app open, with its toast.

use bevy::{
    feathers::{
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody, FeathersDialogClose,
            FeathersDialogFooter, FeathersDialogHeader,
        },
        theme::ThemedText,
    },
    prelude::*,
    ui_widgets::{Activate, RequestClose},
};
use panoply_core::{QuitGate, QuitRequest};

use crate::unsaved::{SaveAssets, SaveSystems, UnsavedAssets};

/// The dialog's root.
#[derive(Component, Default, Clone)]
struct QuitDialog;

/// Set by the dialog's Save: quit once the save has been made, if it succeeds.
#[derive(Resource, Default)]
struct QuitAfterSave(bool);

pub struct QuitDialogPlugin;

impl Plugin for QuitDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuitAfterSave>().add_systems(
            Update,
            (
                hold_quit_while_unsaved.run_if(resource_changed::<UnsavedAssets>),
                open_dialog,
                quit_after_save.after(SaveSystems::Write),
            ),
        );
    }
}

fn hold_quit_while_unsaved(unsaved: Res<UnsavedAssets>, mut gate: ResMut<QuitGate>) {
    gate.held = !unsaved.is_empty();
}

fn open_dialog(
    mut requests: MessageReader<QuitRequest>,
    gate: Res<QuitGate>,
    dialogs: Query<(), With<QuitDialog>>,
    mut commands: Commands,
) {
    if requests.read().count() > 0 && gate.held && dialogs.is_empty() {
        commands.spawn_scene(dialog());
    }
}

/// Reads [`SaveAssets`] after the systems that act on it, so it sees a request on the same frame
/// they do, with the unsaved assets as those writes left them.
fn quit_after_save(
    mut saves: MessageReader<SaveAssets>,
    mut pending: ResMut<QuitAfterSave>,
    unsaved: Res<UnsavedAssets>,
    mut gate: ResMut<QuitGate>,
    mut requests: MessageWriter<QuitRequest>,
) {
    if saves.read().count() == 0 || !pending.0 {
        return;
    }
    pending.0 = false;
    if unsaved.is_empty() {
        gate.held = false;
        requests.write(QuitRequest);
    }
}

fn dialog() -> impl Scene {
    bsn! {
        @FeathersDialog {
            @width: px(360),
            @contents: {Box::new(dialog_contents()) as Box<dyn SceneList>},
        }
        #QuitDialog
        QuitDialog
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
    }
}

/// The contents, in a function of their own, as `goto.rs` explains for its own.
fn dialog_contents() -> impl SceneList {
    bsn_list! {
        @FeathersDialogHeader Children [
            Text("Quit") ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Children [ Text("Save changes before quitting?") ThemedText ]
        --
        @FeathersDialogFooter Children [
            @FeathersButton { @caption: bsn! { Text("Cancel") ThemedText } }
            #Cancel
            on(close_dialog)
            --
            @FeathersButton { @caption: bsn! { Text("Discard") ThemedText } }
            #Discard
            on(discard)
            --
            @FeathersButton {
                @caption: bsn! { Text("Save") ThemedText },
                @variant: ButtonVariant::Primary,
            }
            #SaveAndQuit
            on(save)
        ]
    }
}

fn close_dialog(activate: On<Activate>, mut commands: Commands) {
    commands.trigger(RequestClose {
        source: activate.event_target(),
    });
}

fn discard(
    activate: On<Activate>,
    mut gate: ResMut<QuitGate>,
    mut requests: MessageWriter<QuitRequest>,
    mut commands: Commands,
) {
    gate.held = false;
    requests.write(QuitRequest);
    commands.trigger(RequestClose {
        source: activate.event_target(),
    });
}

fn save(
    activate: On<Activate>,
    mut pending: ResMut<QuitAfterSave>,
    mut saves: MessageWriter<SaveAssets>,
    mut commands: Commands,
) {
    pending.0 = true;
    saves.write(SaveAssets);
    commands.trigger(RequestClose {
        source: activate.event_target(),
    });
}
