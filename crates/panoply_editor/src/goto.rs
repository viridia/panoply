//! The Go To dialog: G opens it, listing the nav points to travel to. A click selects one, and a
//! double-click or Go travels there by `GotoLocation`; Esc, Cancel or a click outside closes it.

use bevy::{
    feathers::{
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody, FeathersDialogClose,
            FeathersDialogFooter, FeathersDialogHeader,
        },
        theme::ThemedText,
    },
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, RequestClose},
};
use bevy_action_map::prelude::*;
use crate::{
    nav_point_list::{nav_point_list, ActivateNavPoint, NavPointList, Travel},
    shell::EditorShellControl,
};

/// Open the Go To dialog.
#[derive(InputAction)]
#[action(path = "editor.goto", output = bool, intent = Button)]
pub struct OpenGotoDialog;

/// The dialog's root.
#[derive(Component, Default, Clone)]
struct GotoDialog;

/// The Go button, disabled while no point is selected.
#[derive(Component, Default, Clone)]
struct GoButton;

pub struct GotoDialogPlugin;

impl Plugin for GotoDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (open_dialog, enable_go));
    }
}

/// While the dialog is open it holds the keys (`widget_keys`), so G there types in the filter.
fn open_dialog(
    input: ContextActions<EditorShellControl>,
    dialogs: Query<(), With<GotoDialog>>,
    mut commands: Commands,
) {
    if input.fired::<OpenGotoDialog>() && dialogs.is_empty() {
        commands.spawn_scene(dialog());
    }
}

fn dialog() -> impl Scene {
    bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: {Box::new(dialog_contents()) as Box<dyn SceneList>},
        }
        #GotoDialog
        GotoDialog
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
        on(travel_to_activated)
    }
}

/// The dialog's contents, in a function of their own: a `bsn_list!` written inside `dialog`'s
/// `bsn!` gave its `#Cancel` the same entity as the root's `#GotoDialog`, which made the barrier
/// and the Cancel button one entity.
fn dialog_contents() -> impl SceneList {
    bsn_list! {
        @FeathersDialogHeader Children [
            Text("Go To") ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Node { height: vh(60), min_height: px(0) }
        Children [ @nav_point_list(true) ]
        --
        @FeathersDialogFooter Children [
            @FeathersButton { @caption: bsn! { Text("Cancel") ThemedText } }
            #Cancel
            on(|activate: On<Activate>, mut commands: Commands| {
                commands.trigger(RequestClose { source: activate.event_target() });
            })
            --
            @FeathersButton {
                @caption: bsn! { Text("Go") ThemedText },
                @variant: ButtonVariant::Primary,
            }
            #Go
            GoButton
            InteractionDisabled
            on(go)
        ]
    }
}

/// The nav point list inside the dialog: the realm panel may hold another.
fn dialog_list<'a>(
    dialog: Entity,
    children: &Query<&Children>,
    lists: &'a Query<&NavPointList>,
) -> Option<&'a NavPointList> {
    children
        .iter_descendants(dialog)
        .find_map(|e| lists.get(e).ok())
}

/// Disables Go while the dialog's list has nothing selected.
fn enable_go(
    dialogs: Query<Entity, With<GotoDialog>>,
    children: Query<&Children>,
    lists: Query<&NavPointList>,
    buttons: Query<(Entity, Has<InteractionDisabled>), With<GoButton>>,
    mut commands: Commands,
) {
    let Some(list) = dialogs
        .iter()
        .find_map(|dialog| dialog_list(dialog, &children, &lists))
    else {
        return;
    };
    for (button, disabled) in &buttons {
        match (list.selected.is_some(), disabled) {
            (true, true) => {
                commands.entity(button).remove::<InteractionDisabled>();
            }
            (false, false) => {
                commands.entity(button).insert(InteractionDisabled);
            }
            _ => {}
        }
    }
}

/// Observer on the Go button: travels to the selected point and closes the dialog.
fn go(
    _activate: On<Activate>,
    dialogs: Query<Entity, With<GotoDialog>>,
    children: Query<&Children>,
    lists: Query<&NavPointList>,
    mut travel: Travel,
    mut commands: Commands,
) {
    for dialog in &dialogs {
        let selected = dialog_list(dialog, &children, &lists).and_then(|l| l.selected.as_ref());
        if let Some(name) = selected {
            travel.to(name);
            commands.entity(dialog).despawn();
        }
    }
}

/// Observer on the dialog: travels to a double-clicked point and closes the dialog.
fn travel_to_activated(activate: On<ActivateNavPoint>, mut travel: Travel, mut commands: Commands) {
    travel.to(&activate.name);
    commands.entity(activate.event_target()).despawn();
}
