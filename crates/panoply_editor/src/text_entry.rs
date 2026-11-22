//! A modal dialog asking for a line of text, such as a new name. OK, or Enter in the box, triggers
//! [`TextEntered`] on the dialog and closes it; the opener observes the event on the entity it
//! spawned. OK is disabled while the text, trimmed, is empty (unless the dialog is clearable), one
//! of the dialog's taken values, or refused by its rule. The taken values are given when it opens: a modal dialog's taken names
//! cannot change while it is open.

use bevy::{
    feathers::{
        constants::icons,
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody, FeathersDialogClose,
            FeathersDialogFooter, FeathersDialogHeader, FeathersTextInput,
            FeathersTextInputContainer, FeathersToolButton,
        },
        display::icon,
        theme::ThemedText,
    },
    input_focus::{AutoFocus, InputFocus},
    platform::collections::HashSet,
    prelude::*,
    text::{EditableText, TextEdit},
    ui::InteractionDisabled,
    ui_widgets::{Activate, RequestClose},
};
use bevy_action_map::prelude::*;

/// The dialog's root, holding the values the text may not take, and the text the box starts with.
#[derive(Component, Default, Clone)]
pub struct TextEntryDialog {
    pub taken: HashSet<String>,
    /// Says why the text, not empty and not taken, cannot be accepted.
    pub rule: Option<TextEntryRule>,
    /// The box's text as the dialog opens, selected, so typing replaces it.
    pub text: String,
    /// Whether the text may be empty: the box has a × that empties it, and an empty text is
    /// accepted, as clearing a value.
    pub clearable: bool,
}

/// A check on the text in a [`TextEntryDialog`], returning why it fails.
pub type TextEntryRule = fn(&str) -> Option<&'static str>;

/// The text box.
#[derive(Component, Default, Clone)]
struct TextEntryInput;

/// The × that empties a clearable dialog's box.
#[derive(Component, Default, Clone)]
struct TextEntryClear;

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
        .add_systems(Update, (fill_entry, validate, accept_on_enter).chain());
    }
}

/// The dialog, titled `title`, its box labeled `label`. Its root is named `#TextEntryDialog`.
pub fn text_entry_dialog(title: &str, label: &str, taken: HashSet<String>) -> impl Scene {
    entry_dialog(title, label, TextEntryDialog { taken, ..default() })
}

/// As [`text_entry_dialog`], with a further check on the text.
pub fn text_entry_dialog_with_rule(
    title: &str,
    label: &str,
    taken: HashSet<String>,
    rule: Option<TextEntryRule>,
) -> impl Scene {
    let dialog = TextEntryDialog {
        taken,
        rule,
        ..default()
    };
    entry_dialog(title, label, dialog)
}

/// As [`text_entry_dialog`], its box starting with `text`, as a rename starts with the name, and
/// clearable: a × empties the box, and OK then clears the value.
pub fn clearable_text_entry_dialog(
    title: &str,
    label: &str,
    taken: HashSet<String>,
    text: String,
) -> impl Scene {
    let dialog = TextEntryDialog {
        taken,
        text,
        clearable: true,
        ..default()
    };
    entry_dialog(title, label, dialog)
}

fn entry_dialog(title: &str, label: &str, dialog: TextEntryDialog) -> impl Scene {
    let (title, label) = (title.to_string(), label.to_string());
    let clearable = dialog.clearable;
    bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: {
                Box::new(dialog_contents(title.clone(), label.clone(), clearable))
                    as Box<dyn SceneList>
            },
        }
        #TextEntryDialog
        TextEntryDialog {
            taken: {dialog.taken.clone()},
            rule: {dialog.rule},
            text: {dialog.text.clone()},
            clearable: {clearable},
        }
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
    }
}

/// The contents, in a function of their own, as `goto.rs` explains for its own.
fn dialog_contents(title: String, label: String, clearable: bool) -> impl SceneList {
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
            @entry_box(clearable)
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

/// The text box, and in a clearable dialog the × after it, as the chooser's filter box has one.
fn entry_box(clearable: bool) -> Box<dyn Scene> {
    if clearable {
        Box::new(bsn! {
            @FeathersTextInputContainer Children [
                @FeathersTextInput
                #Entry
                TextEntryInput
                AutoFocus
                --
                @FeathersToolButton {
                    @variant: ButtonVariant::Plain,
                    @caption: bsn! { @icon(icons::X) },
                }
                #Clear
                TextEntryClear
                on(clear_entry)
            ]
        })
    } else {
        Box::new(bsn! {
            @FeathersTextInputContainer Children [
                @FeathersTextInput
                #Entry
                TextEntryInput
                AutoFocus
            ]
        })
    }
}

/// Observer on the ×: empties the box.
fn clear_entry(
    activate: On<Activate>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    dialogs: Query<(), With<TextEntryDialog>>,
    mut inputs: Query<&mut EditableText, With<TextEntryInput>>,
) {
    let Some(root) = parents
        .iter_ancestors(activate.event_target())
        .find(|&e| dialogs.contains(e))
    else {
        return;
    };
    if let Some(input) = children
        .iter_descendants(root)
        .find(|&e| inputs.contains(e))
    {
        inputs.get_mut(input).unwrap().clear();
    }
}

/// Why `text` cannot be accepted by `dialog`, if it cannot: `Some("")` when it is empty and the
/// dialog not clearable, which needs no hint.
fn rejection(dialog: &TextEntryDialog, text: &str) -> Option<&'static str> {
    if text.is_empty() {
        (!dialog.clearable).then_some("")
    } else if dialog.taken.contains(text) {
        Some("Name in use")
    } else {
        dialog.rule.and_then(|rule| rule(text))
    }
}

/// Puts the dialog's starting text in its box as the box appears, selected.
fn fill_entry(
    mut inputs: Query<(Entity, &mut EditableText), Added<TextEntryInput>>,
    parents: Query<&ChildOf>,
    dialogs: Query<&TextEntryDialog>,
) {
    for (input, mut editable) in &mut inputs {
        let text = parents
            .iter_ancestors(input)
            .find_map(|e| dialogs.get(e).ok())
            .map(|dialog| dialog.text.as_str())
            .unwrap_or_default();
        if !text.is_empty() {
            editable.editor.set_text(text);
            editable.queue_edit(TextEdit::SelectAll);
        }
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
