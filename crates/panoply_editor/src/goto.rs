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
use panoply_core::{GotoLocation, NavPointsAsset, NavPointsHandleResource};

use crate::{
    nav_point_list::{nav_point_list, ActivateNavPoint, NavPointList},
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
        Children [ @nav_point_list() ]
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

/// Disables Go while the list has nothing selected.
fn enable_go(
    lists: Query<&NavPointList, Changed<NavPointList>>,
    buttons: Query<(Entity, Has<InteractionDisabled>), With<GoButton>>,
    mut commands: Commands,
) {
    let Some(list) = lists.iter().next() else {
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

/// Observer on the Go button: travels to the selected point.
fn go(
    _activate: On<Activate>,
    lists: Query<&NavPointList>,
    dialogs: Query<Entity, With<GotoDialog>>,
    mut travel: Travel,
) {
    let Some(name) = lists.iter().find_map(|list| list.selected.clone()) else {
        return;
    };
    for dialog in &dialogs {
        travel.to(&name, dialog);
    }
}

/// Observer on the dialog: travels to a double-clicked point.
fn travel_to_activated(activate: On<ActivateNavPoint>, mut travel: Travel) {
    travel.to(&activate.name, activate.event_target());
}

#[derive(bevy::ecs::system::SystemParam)]
struct Travel<'w, 's> {
    handle: Res<'w, NavPointsHandleResource>,
    assets: Res<'w, Assets<NavPointsAsset>>,
    gotos: MessageWriter<'w, GotoLocation>,
    commands: Commands<'w, 's>,
}

impl Travel<'_, '_> {
    /// Moves the viewpoint to the point named `name`, and closes `dialog`.
    fn to(&mut self, name: &str, dialog: Entity) {
        let point = self
            .assets
            .get(&self.handle.0)
            .and_then(|points| points.0.iter().find(|p| p.name == name));
        if let Some(point) = point {
            self.gotos.write(GotoLocation {
                realm: point.realm.clone(),
                position: point.pos,
                ..default()
            });
        }
        self.commands.entity(dialog).despawn();
    }
}
