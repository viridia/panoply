//! Keyboard input for focused widgets, delivered by the action mapper rather than by Bevy's
//! `InputDispatchPlugin`, which the app disables. One arbiter then decides where every key goes:
//! the mapper's priority and consumption decide whether a key reaches a widget, a game action, or
//! both, and Bevy's widgets (feathers included) receive the same `FocusedInput<KeyboardInput>` they
//! always did, unchanged.
//!
//! Where a key goes depends on what has focus, told by marker components on the focused entity
//! and its ancestors. The plugin makes each marker a required component of its widget, so every
//! `TextInput` carries [`TextInputFocus`] and every `ModalDialog` [`ModalDialogFocus`] without the
//! widget's own scene naming them. A modal takes focus as it opens, as ARIA asks of a dialog.
//!
//! Two contexts bind every button and trigger `FocusedInput` at the focused entity, or at the
//! primary window when nothing has focus, as Bevy's dispatch does:
//!
//! - [`WidgetKeysClaim`], above every other context, consumes what it takes. It is active while a
//!   text input has focus, or anything inside a modal dialog does, so typing reaches only the
//!   widget and a modal traps the keys as it traps focus: a letter bound to a game action types
//!   instead.
//! - [`WidgetKeysFallthrough`], below every other context, consumes nothing. A widget kind with no
//!   context of its own gets each key that nothing above consumed, as it would from Bevy's
//!   dispatch, beside any action bound to it. A kind with a context of its own consumes what it
//!   handles, so that a focused `bevy_ui_widgets::Button` is not activated by both.
//!
//! Unlike Bevy's dispatch, key releases are not delivered: a class binding fires only while its
//! control reads as pressed. No widget in `bevy_ui_widgets` or feathers reads a release. IME
//! events do not pass through here; text input reads them itself.
//!
//! Nothing here names a type of this game, so the module can move to `bevy_action_map`.

use bevy::{
    ecs::entity::Entities,
    input::{gamepad::GamepadButtonChangedEvent, mouse::MouseWheel, InputSystems},
    input_focus::{
        dispatch_focused_input, FocusCause, FocusedInput, InputFocus, InputFocusSystems,
    },
    prelude::*,
    ui_widgets::{ModalDialog, TextInput},
    window::PrimaryWindow,
};
use bevy_action_map::prelude::*;

/// Marks a text input, which takes every key while it has focus.
#[derive(Component, Default, Clone, Copy)]
pub struct TextInputFocus;

/// Marks a modal dialog, which takes every key while focus is inside it.
#[derive(Component, Default, Clone, Copy)]
pub struct ModalDialogFocus;

/// Gives keys to the focused widget alone, while [`widget_claims_keys`].
#[derive(InputContext)]
#[context(path = "widget_keys.claim", tick = Render, priority = 1000)]
pub struct WidgetKeysClaim;

/// Gives the focused widget every key nothing above consumed.
#[derive(InputContext)]
#[context(path = "widget_keys.fallthrough", tick = Render, priority = -1000)]
pub struct WidgetKeysFallthrough;

/// A key [`WidgetKeysClaim`] took.
pub struct ClaimedWidgetKey;

impl ClassBinding for ClaimedWidgetKey {
    const PATH: &'static str = "widget_keys.claimed";
}

/// A key [`WidgetKeysFallthrough`] passed on.
pub struct UnclaimedWidgetKey;

impl ClassBinding for UnclaimedWidgetKey {
    const PATH: &'static str = "widget_keys.unclaimed";
}

pub struct WidgetKeysPlugin;

impl Plugin for WidgetKeysPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<TextInput, TextInputFocus>();
        app.register_required_components::<ModalDialog, ModalDialogFocus>();
        app.add_observer(focus_opened_modal)
            .add_context::<WidgetKeysClaim>(|context| {
                context.active_if(widget_claims_keys);
                context
                    .bind_class::<ClaimedWidgetKey>(ControlClass::AnyButton)
                    .consume();
            })
            .add_context::<WidgetKeysFallthrough>(|context| {
                context.bind_class::<UnclaimedWidgetKey>(ControlClass::AnyButton);
            })
            .add_observer(deliver_widget_key::<ClaimedWidgetKey>)
            .add_observer(deliver_widget_key::<UnclaimedWidgetKey>)
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("WidgetKeysClaim"), WidgetKeysClaim));
                commands.spawn((Name::new("WidgetKeysFallthrough"), WidgetKeysFallthrough));
            })
            // The rest of what `InputDispatchPlugin` delivered, as it did. Nothing in Bevy's
            // widgets reads these today, but a widget of a game's own may.
            .add_systems(
                PreUpdate,
                (
                    dispatch_focused_input::<GamepadButtonChangedEvent>,
                    dispatch_focused_input::<MouseWheel>,
                )
                    .chain()
                    .in_set(InputFocusSystems::Dispatch)
                    .after(InputSystems),
            );
    }
}

/// Whether the focused widget takes every key: a text input, or anything inside a modal dialog,
/// the dialog included.
pub fn widget_claims_keys(
    focus: Res<InputFocus>,
    texts: Query<(), With<TextInputFocus>>,
    modals: Query<(), With<ModalDialogFocus>>,
    parents: Query<&ChildOf>,
) -> bool {
    let Some(focused) = focus.get() else {
        return false;
    };
    texts.contains(focused)
        || std::iter::once(focused)
            .chain(parents.iter_ancestors(focused))
            .any(|entity| modals.contains(entity))
}

/// Focuses a modal dialog as it opens, unless focus is already inside it (a widget's autofocus).
/// Bevy's modal moves focus to a widget within a moment later, which keeps it inside the dialog;
/// until then the dialog holds it, so no key reaches what lies behind.
fn focus_opened_modal(
    add: On<Add<ModalDialogFocus>>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let dialog = add.event_target();
    let inside = focus.get().is_some_and(|focused| {
        focused == dialog || parents.iter_ancestors(focused).any(|e| e == dialog)
    });
    if !inside {
        focus.set(dialog, FocusCause::Navigated);
    }
}

/// Delivers a key to the focused entity, or to the primary window if nothing has focus or the
/// focused entity is gone, as Bevy's `dispatch_focused_input` does.
fn deliver_widget_key<A: ClassBinding>(
    fired: On<ClassFired<A>>,
    mut focus: ResMut<InputFocus>,
    windows: Query<Entity, With<PrimaryWindow>>,
    entities: &Entities,
    mut commands: Commands,
) {
    let RawEvent::Keyboard(input) = &fired.event else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let target = match focus.get() {
        Some(entity) if entities.contains(entity) => entity,
        Some(_) => {
            focus.clear();
            window
        }
        None => window,
    };
    commands.trigger(FocusedInput::new(target, input.clone(), window));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::{
        keyboard::{Key, KeyboardInput, NativeKeyCode},
        ButtonState, InputPlugin,
    };
    use bevy::input_focus::InputFocusPlugin;

    #[derive(InputAction)]
    #[action(path = "tests.walk", output = bool, intent = Button)]
    struct Walk;

    #[derive(InputAction)]
    #[action(path = "tests.press", output = bool, intent = Button)]
    struct Press;

    /// A game's own context, at the default priority.
    #[derive(InputContext)]
    #[context(path = "tests.on_foot", tick = Render)]
    struct OnFoot;

    /// A game widget kind's context, which consumes what it handles.
    #[derive(InputContext)]
    #[context(path = "tests.button_focused", tick = Render, priority = 20)]
    struct ButtonFocused;

    /// Keys delivered as `FocusedInput`, and to which entity; and whether `Walk` fired.
    #[derive(Resource, Default)]
    struct Seen {
        delivered: Vec<(Entity, KeyCode)>,
        walked: bool,
    }

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            InputFocusPlugin,
            ActionMapPlugin,
            WidgetKeysPlugin,
        ));
        app.add_context::<OnFoot>(|context| {
            context.bind::<Walk>(KeyCode::KeyW);
        });
        app.add_context::<ButtonFocused>(|context| {
            context.bind::<Press>(KeyCode::Enter).consume();
        });
        app.world_mut().spawn(OnFoot);
        app.world_mut().spawn(ButtonFocused);
        app.init_resource::<Seen>();
        app.add_observer(
            // The event bubbles up to the window, as from Bevy's dispatch; record where it began.
            |input: On<FocusedInput<KeyboardInput>>, mut seen: ResMut<Seen>| {
                if input.event_target() == input.original_event_target() {
                    seen.delivered
                        .push((input.focused_entity, input.input.key_code));
                }
            },
        );
        app.add_systems(
            Update,
            |input: ContextActions<OnFoot>, mut seen: ResMut<Seen>| {
                seen.walked |= input.fired::<Walk>();
            },
        );
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        // Startup spawns the contexts.
        app.update();
        (app, window)
    }

    fn send(app: &mut App, key_code: KeyCode, state: ButtonState, repeat: bool) -> Seen {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        std::mem::take(&mut *app.world_mut().resource_mut::<Seen>())
    }

    fn press(app: &mut App, key_code: KeyCode) -> Seen {
        let seen = send(app, key_code, ButtonState::Pressed, false);
        send(app, key_code, ButtonState::Released, false);
        seen
    }

    fn focus(app: &mut App, entity: Entity) {
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(entity, bevy::input_focus::FocusCause::Navigated);
    }

    #[test]
    fn without_focus_a_key_reaches_the_window_and_the_game() {
        let (mut app, window) = app();
        let seen = press(&mut app, KeyCode::KeyW);
        assert_eq!(seen.delivered, [(window, KeyCode::KeyW)]);
        assert!(seen.walked);
    }

    #[test]
    fn a_focused_text_input_takes_keys_from_the_game() {
        let (mut app, _) = app();
        let field = app.world_mut().spawn(TextInput).id();
        focus(&mut app, field);
        let seen = press(&mut app, KeyCode::KeyW);
        assert_eq!(seen.delivered, [(field, KeyCode::KeyW)]);
        assert!(!seen.walked, "W walked while a text input had focus");
    }

    #[test]
    fn an_opened_modal_takes_focus_and_keys() {
        let (mut app, _) = app();
        let dialog = app.world_mut().spawn(ModalDialog).id();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(dialog));
        let seen = press(&mut app, KeyCode::KeyW);
        assert_eq!(seen.delivered, [(dialog, KeyCode::KeyW)]);
        assert!(!seen.walked, "W walked while a modal was open");
    }

    #[test]
    fn focus_inside_a_modal_takes_keys() {
        let (mut app, _) = app();
        let dialog = app.world_mut().spawn(ModalDialog).id();
        let button = app.world_mut().spawn(ChildOf(dialog)).id();
        focus(&mut app, button);
        let seen = press(&mut app, KeyCode::KeyW);
        assert_eq!(seen.delivered, [(button, KeyCode::KeyW)]);
        assert!(!seen.walked, "W walked with focus inside a modal");
    }

    #[test]
    fn an_opened_modal_keeps_focus_already_inside_it() {
        let (mut app, _) = app();
        let dialog = app.world_mut().spawn_empty().id();
        let field = app.world_mut().spawn(ChildOf(dialog)).id();
        focus(&mut app, field);
        app.world_mut().entity_mut(dialog).insert(ModalDialog);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    }

    #[test]
    fn other_focus_shares_keys_with_the_game() {
        let (mut app, _) = app();
        let checkbox = app.world_mut().spawn_empty().id();
        focus(&mut app, checkbox);
        let seen = press(&mut app, KeyCode::KeyW);
        assert_eq!(seen.delivered, [(checkbox, KeyCode::KeyW)]);
        assert!(seen.walked);
    }

    #[test]
    fn a_key_a_widget_context_consumes_is_not_delivered_again() {
        let (mut app, window) = app();
        let seen = press(&mut app, KeyCode::Enter);
        assert!(
            !seen.delivered.contains(&(window, KeyCode::Enter)),
            "Enter was both handled and delivered: {:?}",
            seen.delivered
        );
    }

    #[test]
    fn repeats_are_delivered_and_releases_are_not() {
        let (mut app, _) = app();
        let field = app.world_mut().spawn(TextInput).id();
        focus(&mut app, field);
        let first = send(&mut app, KeyCode::Backspace, ButtonState::Pressed, false);
        let repeat = send(&mut app, KeyCode::Backspace, ButtonState::Pressed, true);
        let release = send(&mut app, KeyCode::Backspace, ButtonState::Released, false);
        assert_eq!(first.delivered, [(field, KeyCode::Backspace)]);
        assert_eq!(repeat.delivered, [(field, KeyCode::Backspace)]);
        assert_eq!(release.delivered, []);
    }

    #[test]
    fn a_key_without_a_named_code_is_delivered() {
        let (mut app, window) = app();
        let code = KeyCode::Unidentified(NativeKeyCode::Unidentified);
        let seen = press(&mut app, code);
        assert_eq!(seen.delivered, [(window, code)]);
    }

    #[test]
    fn a_despawned_focus_falls_back_to_the_window() {
        let (mut app, window) = app();
        let gone = app.world_mut().spawn_empty().id();
        focus(&mut app, gone);
        app.world_mut().despawn(gone);
        let seen = press(&mut app, KeyCode::KeyA);
        assert_eq!(seen.delivered, [(window, KeyCode::KeyA)]);
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
    }
}
