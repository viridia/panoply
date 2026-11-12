//! Committing a text input's text: Bevy's text input reports each keystroke, not when an edit is
//! done. For every [`TextInput`], Enter or losing focus triggers a [`ValueChange<String>`] on it,
//! `is_final` set, carrying its text, and Escape puts back the text it had when it gained focus.
//! A text that has not changed since focus was gained, or since the last commit, is not committed
//! again, so Enter and then leaving the field commit once. A multi-line input commits only on
//! losing focus, since Enter is a line break there.
//!
//! Free of the editor's types: Bevy may gain the same, and then this module goes.

use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input_focus::{FocusGained, FocusLost, FocusedInput},
    prelude::*,
    text::{EditableText, TextEdit},
    ui_widgets::{TextInput, ValueChange},
};

pub struct TextCommitPlugin;

impl Plugin for TextCommitPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(begin_edit)
            .add_observer(commit_on_key)
            .add_observer(commit_on_focus_lost);
    }
}

/// A focused text input's edit: the text last committed, or held when focus was gained, and
/// whether Escape has put that text back.
#[derive(Component)]
struct TextEdited {
    committed: String,
    reverted: bool,
}

/// Remembers a text input's text as it gains focus.
fn begin_edit(
    gained: On<FocusGained>,
    inputs: Query<&EditableText, With<TextInput>>,
    mut commands: Commands,
) {
    let entity = gained.event_target();
    if let Ok(text) = inputs.get(entity) {
        commands.entity(entity).insert(TextEdited {
            committed: text.value().into_owned(),
            reverted: false,
        });
    }
}

/// Commits a single-line input's text on Enter; puts back its text on Escape.
fn commit_on_key(
    key: On<FocusedInput<KeyboardInput>>,
    mut inputs: Query<(&mut EditableText, &mut TextEdited), With<TextInput>>,
    mut commands: Commands,
) {
    if !key.input.state.is_pressed() {
        return;
    }
    let entity = key.focused_entity;
    let Ok((mut text, mut edited)) = inputs.get_mut(entity) else {
        return;
    };
    match key.input.logical_key {
        Key::Enter if !text.allow_newlines => {
            let value = text.value().into_owned();
            if value != edited.committed {
                edited.committed = value.clone();
                commands.trigger(ValueChange {
                    source: entity,
                    value,
                    is_final: true,
                });
            }
        }
        Key::Escape => {
            // The input loses focus on Escape too, which then commits nothing.
            if text.value() != edited.committed {
                text.editor.set_text(&edited.committed);
                text.queue_edit(TextEdit::TextEnd(false));
            }
            edited.reverted = true;
        }
        _ => {}
    }
}

/// Commits a text input's text as it loses focus, unless Escape put it back.
fn commit_on_focus_lost(
    lost: On<FocusLost>,
    inputs: Query<(&EditableText, &TextEdited), With<TextInput>>,
    mut commands: Commands,
) {
    let entity = lost.event_target();
    let Ok((text, edited)) = inputs.get(entity) else {
        return;
    };
    let value = text.value().into_owned();
    if !edited.reverted && value != edited.committed {
        commands.trigger(ValueChange {
            source: entity,
            value,
            is_final: true,
        });
    }
    commands.entity(entity).remove::<TextEdited>();
}

#[cfg(test)]
mod tests {
    use bevy::{input::ButtonState, input_focus::FocusCause};

    use super::*;

    #[derive(Resource, Default)]
    struct Committed(Vec<String>);

    /// An app with a text input that has gained focus showing "old", then been typed into to
    /// show `typed`.
    fn editing(typed: &str) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(TextCommitPlugin)
            .init_resource::<Committed>()
            .add_observer(
                |change: On<ValueChange<String>>, mut committed: ResMut<Committed>| {
                    assert!(change.is_final);
                    committed.0.push(change.value.clone());
                },
            );
        let input = app
            .world_mut()
            .spawn((TextInput, EditableText::new("old")))
            .id();
        app.world_mut().trigger(FocusGained {
            entity: input,
            cause: FocusCause::Pressed,
        });
        app.update();
        app.world_mut()
            .get_mut::<EditableText>(input)
            .unwrap()
            .editor
            .set_text(typed);
        (app, input)
    }

    fn press(app: &mut App, input: Entity, key: Key) {
        let window = app.world_mut().spawn_empty().id();
        app.world_mut().trigger(FocusedInput::new(
            input,
            KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: key,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            },
            window,
        ));
        app.update();
    }

    fn lose_focus(app: &mut App, input: Entity) {
        app.world_mut().trigger(FocusLost { entity: input });
        app.update();
    }

    fn committed(app: &App) -> &[String] {
        &app.world().resource::<Committed>().0
    }

    fn text(app: &App, input: Entity) -> String {
        app.world()
            .get::<EditableText>(input)
            .unwrap()
            .value()
            .into_owned()
    }

    #[test]
    fn enter_then_leaving_commits_once() {
        let (mut app, input) = editing("new");
        press(&mut app, input, Key::Enter);
        lose_focus(&mut app, input);
        assert_eq!(committed(&app), ["new"]);
    }

    #[test]
    fn leaving_commits() {
        let (mut app, input) = editing("new");
        lose_focus(&mut app, input);
        assert_eq!(committed(&app), ["new"]);
    }

    #[test]
    fn an_unchanged_text_is_not_committed() {
        let (mut app, input) = editing("old");
        press(&mut app, input, Key::Enter);
        lose_focus(&mut app, input);
        assert!(committed(&app).is_empty());
    }

    #[test]
    fn escape_reverts_without_committing() {
        let (mut app, input) = editing("new");
        press(&mut app, input, Key::Escape);
        lose_focus(&mut app, input);
        assert!(committed(&app).is_empty());
        assert_eq!(text(&app, input), "old");
    }
}
