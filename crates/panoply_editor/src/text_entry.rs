//! A modal dialog asking for a line of text, such as a new name. OK, or Enter in the box, triggers
//! [`TextEntered`] on the dialog and closes it; the opener observes the event on the entity it
//! spawned. OK is disabled while the text, trimmed, is empty, one of the dialog's taken values, or
//! refused by its rule. The taken values are given when it opens: a modal dialog's taken names
//! cannot change while it is open.

use bevy::{
    feathers::{
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody, FeathersDialogClose,
            FeathersDialogFooter, FeathersDialogHeader, FeathersTextInput,
            FeathersTextInputContainer,
        },
        theme::ThemedText,
    },
    input_focus::{AutoFocus, InputFocus},
    platform::collections::HashSet,
    prelude::*,
    text::EditableText,
    ui::InteractionDisabled,
    ui_widgets::{Activate, RequestClose},
};
use bevy_action_map::prelude::*;

/// The dialog's root, holding the values the text may not take.
#[derive(Component, Default, Clone)]
pub struct TextEntryDialog {
    pub taken: HashSet<String>,
    /// Says why the text, not empty and not taken, cannot be accepted.
    pub rule: Option<TextEntryRule>,
}

/// A check on the text in a [`TextEntryDialog`], returning why it fails.
pub type TextEntryRule = fn(&str) -> Option<&'static str>;

/// The text box.
#[derive(Component, Default, Clone)]
struct TextEntryInput;

/// The line under the box saying why OK is disabled.
#[derive(Component, Default, Clone)]
struct TextEntryHint;

/// The OK button.
#[derive(Component, Default, Clone)]
struct TextEntryOk;

/// The text accepted, triggered on the dialog as it closes.
#[derive(EntityEvent, Clone, Debug)]
pub struct TextEntered {
    #[event_target]
    pub dialog: Entity,
    /// The text, trimmed.
    pub text: String,
}

/// The box's keys, above `widget_keys`' claim on a focused text input.
#[derive(InputContext)]
#[context(path = "editor.text_entry", tick = Render, priority = 1100)]
pub struct TextEntryKeys;

/// Accept the text.
#[derive(InputAction)]
#[action(path = "editor.text_entry.accept", output = bool, intent = Button)]
pub struct AcceptTextEntry;

pub struct TextEntryPlugin;

impl Plugin for TextEntryPlugin {
    fn build(&self, app: &mut App) {
        app.add_context::<TextEntryKeys>(|context| {
            context.active_if(input_focused);
            context.bind::<AcceptTextEntry>(KeyCode::Enter).consume();
            context
                .bind::<AcceptTextEntry>(KeyCode::NumpadEnter)
                .consume();
        })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Name::new("TextEntryKeys"), TextEntryKeys));
        })
        .add_systems(Update, (validate, accept_on_enter));
    }
}

/// The dialog, titled `title`, its box labeled `label`. Its root is named `#TextEntryDialog`.
pub fn text_entry_dialog(title: &str, label: &str, taken: HashSet<String>) -> impl Scene {
    text_entry_dialog_with_rule(title, label, taken, None)
}

/// As [`text_entry_dialog`], with a further check on the text.
pub fn text_entry_dialog_with_rule(
    title: &str,
    label: &str,
    taken: HashSet<String>,
    rule: Option<TextEntryRule>,
) -> impl Scene {
    let (title, label) = (title.to_string(), label.to_string());
    bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: {
                Box::new(dialog_contents(title.clone(), label.clone())) as Box<dyn SceneList>
            },
        }
        #TextEntryDialog
        TextEntryDialog { taken: {taken.clone()}, rule: {rule} }
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
    }
}

/// The contents, in a function of their own, as `goto.rs` explains for its own.
fn dialog_contents(title: String, label: String) -> impl SceneList {
    bsn_list! {
        @FeathersDialogHeader Children [
            Text({title.clone()}) ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Node { flex_direction: FlexDirection::Column, row_gap: px(4) }
        Children [
            Text({label.clone()}) ThemedText
            --
            @FeathersTextInputContainer Children [
                @FeathersTextInput
                #Entry
                TextEntryInput
                AutoFocus
            ]
            --
            Text("") ThemedText
            #Hint
            TextEntryHint
        ]
        --
        @FeathersDialogFooter Children [
            @FeathersButton { @caption: bsn! { Text("Cancel") ThemedText } }
            #Cancel
            on(|activate: On<Activate>, mut commands: Commands| {
                commands.trigger(RequestClose { source: activate.event_target() });
            })
            --
            @FeathersButton {
                @caption: bsn! { Text("OK") ThemedText },
                @variant: ButtonVariant::Primary,
            }
            #Ok
            TextEntryOk
            InteractionDisabled
            on(accept_on_ok)
        ]
    }
}

/// Why `text` cannot be accepted by `dialog`, if it cannot: `Some("")` when it is empty, which
/// needs no hint.
fn rejection(dialog: &TextEntryDialog, text: &str) -> Option<&'static str> {
    if text.is_empty() {
        Some("")
    } else if dialog.taken.contains(text) {
        Some("Name in use")
    } else {
        dialog.rule.and_then(|rule| rule(text))
    }
}

/// As the text changes, enables OK while it can be accepted, and says why not.
fn validate(
    inputs: Query<(Entity, &EditableText), (With<TextEntryInput>, Changed<EditableText>)>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    dialogs: Query<&TextEntryDialog>,
    oks: Query<Has<InteractionDisabled>, With<TextEntryOk>>,
    mut hints: Query<&mut Text, With<TextEntryHint>>,
    mut commands: Commands,
) {
    for (input, text) in &inputs {
        let Some((root, dialog)) = parents
            .iter_ancestors(input)
            .find_map(|e| dialogs.get(e).ok().map(|d| (e, d)))
        else {
            continue;
        };
        let rejection = rejection(dialog, text.value().trim());
        for e in children.iter_descendants(root) {
            if let Ok(disabled) = oks.get(e) {
                match (rejection.is_none(), disabled) {
                    (true, true) => {
                        commands.entity(e).remove::<InteractionDisabled>();
                    }
                    (false, false) => {
                        commands.entity(e).insert(InteractionDisabled);
                    }
                    _ => {}
                }
            }
            if let Ok(mut hint) = hints.get_mut(e) {
                hint.set_if_neq(Text(rejection.unwrap_or("").to_string()));
            }
        }
    }
}

/// Observer on OK.
fn accept_on_ok(activate: On<Activate>, mut accept: Accept) {
    accept.from(activate.event_target());
}

/// Enter in the box accepts its text.
fn accept_on_enter(
    input: ContextActions<TextEntryKeys>,
    focus: Res<InputFocus>,
    mut accept: Accept,
) {
    if input.fired::<AcceptTextEntry>() {
        if let Some(focused) = focus.get() {
            accept.from(focused);
        }
    }
}

fn input_focused(focus: Res<InputFocus>, inputs: Query<(), With<TextEntryInput>>) -> bool {
    focus.get().is_some_and(|focused| inputs.contains(focused))
}

#[derive(bevy::ecs::system::SystemParam)]
struct Accept<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    dialogs: Query<'w, 's, &'static TextEntryDialog>,
    inputs: Query<'w, 's, &'static EditableText, With<TextEntryInput>>,
    commands: Commands<'w, 's>,
}

impl Accept<'_, '_> {
    /// Accepts the text of the dialog holding `entity`, if it can be: triggers [`TextEntered`]
    /// and closes the dialog.
    fn from(&mut self, entity: Entity) {
        let Some((root, dialog)) = self
            .parents
            .iter_ancestors(entity)
            .find_map(|e| self.dialogs.get(e).ok().map(|d| (e, d)))
        else {
            return;
        };
        let Some(text) = self
            .children
            .iter_descendants(root)
            .find_map(|e| self.inputs.get(e).ok())
        else {
            return;
        };
        let text = text.value().trim().to_string();
        if rejection(dialog, &text).is_some() {
            return;
        }
        self.commands.trigger(TextEntered { dialog: root, text });
        self.commands.entity(root).despawn();
    }
}
