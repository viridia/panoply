//! The editor's undo history: one stack for every asset and mode, so edits undo in the order they
//! were made (D58). An action names what it changes by asset and key, never by `Entity` (D49).
//! Undoing or redoing returns to the mode the edit was made in, so the change lands where it can be
//! seen; switching modes is not itself an edit, and leaves the history alone.

use std::borrow::Cow;

use bevy::{prelude::*, settings::SaveSettingsDeferred};
use bevy_action_map::prelude::*;
use panoply_core::GameMode;

use crate::{
    mode_selector::{EditorMode, EditorModeSettings},
    shell::SAVE_DELAY,
    toast::ShowToast,
};

/// A change that can be applied to the world, and that yields its inverse: applying an undo gives
/// the redo, and applying that gives the undo again.
pub trait UndoAction: Send + Sync + 'static {
    /// Makes the change and returns the action that reverses it.
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction>;

    /// What applying this does, as "Remove nav point inn".
    fn label(&self) -> Cow<'static, str>;
}

/// An action and the mode its edit was made in.
struct UndoEntry {
    mode: EditorMode,
    action: Box<dyn UndoAction>,
}

impl UndoEntry {
    /// Returns to the entry's mode, applies its action, and returns the inverse entry.
    fn apply(self, world: &mut World, direction: &str) -> UndoEntry {
        let message = format!("{direction}: {}", self.action.label());
        info!("{message}");
        world.write_message(ShowToast(message.into()));
        show_mode(world, self.mode);
        UndoEntry {
            mode: self.mode,
            action: self.action.apply(world),
        }
    }
}

/// Chooses `mode` as the mode selector does: the state follows the setting.
fn show_mode(world: &mut World, mode: EditorMode) {
    let mut settings = world.resource_mut::<EditorModeSettings>();
    if settings.editor_mode != mode {
        settings.editor_mode = mode;
        world.commands().queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// The undo and redo history.
#[derive(Resource, Default)]
pub struct UndoStack {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

impl UndoStack {
    /// Records an edit just made in `mode`, given the action that reverses it. Clears the redo
    /// history.
    pub fn push(&mut self, mode: EditorMode, action: impl UndoAction) {
        self.undo.push(UndoEntry {
            mode,
            action: Box::new(action),
        });
        self.redo.clear();
    }

    /// Reverses the latest edit, if there is one.
    pub fn undo(&mut self, world: &mut World) {
        if let Some(entry) = self.undo.pop() {
            self.redo.push(entry.apply(world, "Undo"));
        }
    }

    /// Makes again the latest edit undone, if there is one.
    pub fn redo(&mut self, world: &mut World) {
        if let Some(entry) = self.redo.pop() {
            self.undo.push(entry.apply(world, "Redo"));
        }
    }
}

/// The history's keys, while editing. A focused text field or a modal dialog holds the keys
/// (`widget_keys`), so these do not reach the history from there.
#[derive(InputContext)]
#[context(path = "editor.history", tick = Render)]
pub struct EditorHistoryKeys;

/// Undo the latest edit.
#[derive(InputAction)]
#[action(path = "editor.history.undo", output = bool, intent = Button)]
pub struct UndoEdit;

/// Redo the latest edit undone.
#[derive(InputAction)]
#[action(path = "editor.history.redo", output = bool, intent = Button)]
pub struct RedoEdit;

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UndoStack>()
            .add_context::<EditorHistoryKeys>(|context| {
                context.active_in_state(GameMode::Edit);
                // Control on every platform, and Command on macOS; Shift makes a binding more
                // specific, so it takes over from undo.
                for modifier in [ModifierKey::Ctrl, ModifierKey::Super] {
                    context.bind::<UndoEdit>(KeyCode::KeyZ).with(modifier);
                    context
                        .bind::<RedoEdit>(KeyCode::KeyZ)
                        .with(modifier)
                        .with(ModifierKey::Shift);
                }
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("EditorHistoryKeys"), EditorHistoryKeys));
            })
            .add_systems(Update, history_keys);
    }
}

fn history_keys(input: ContextActions<EditorHistoryKeys>, mut commands: Commands) {
    if input.fired::<UndoEdit>() {
        commands.queue(|world: &mut World| {
            world.resource_scope(|world, mut stack: Mut<UndoStack>| stack.undo(world));
        });
    }
    if input.fired::<RedoEdit>() {
        commands.queue(|world: &mut World| {
            world.resource_scope(|world, mut stack: Mut<UndoStack>| stack.redo(world));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sets a counter, and yields the action that sets it back.
    struct SetCount(u32);

    #[derive(Resource, Default)]
    struct Count(u32);

    impl UndoAction for SetCount {
        fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
            let mut count = world.resource_mut::<Count>();
            let previous = count.0;
            count.0 = self.0;
            Box::new(SetCount(previous))
        }

        fn label(&self) -> Cow<'static, str> {
            "Set count".into()
        }
    }

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<Count>();
        world.init_resource::<EditorModeSettings>();
        world
    }

    /// Sets the counter as an edit in `mode` would, and records how to reverse it.
    fn edit(world: &mut World, stack: &mut UndoStack, mode: EditorMode, value: u32) {
        let mut count = world.resource_mut::<Count>();
        let previous = count.0;
        count.0 = value;
        stack.push(mode, SetCount(previous));
    }

    fn count(world: &World) -> u32 {
        world.resource::<Count>().0
    }

    #[test]
    fn undo_and_redo_reverse_each_other_in_order() {
        let mut world = world();
        let mut stack = UndoStack::default();
        edit(&mut world, &mut stack, EditorMode::Realm, 1);
        edit(&mut world, &mut stack, EditorMode::Realm, 2);

        stack.undo(&mut world);
        assert_eq!(count(&world), 1);
        stack.undo(&mut world);
        assert_eq!(count(&world), 0);
        stack.redo(&mut world);
        assert_eq!(count(&world), 1);
        stack.redo(&mut world);
        assert_eq!(count(&world), 2);
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut world = world();
        let mut stack = UndoStack::default();
        edit(&mut world, &mut stack, EditorMode::Realm, 1);
        stack.undo(&mut world);
        edit(&mut world, &mut stack, EditorMode::Realm, 5);
        stack.redo(&mut world);
        assert_eq!(count(&world), 5);
    }

    #[test]
    fn an_empty_history_does_nothing() {
        let mut world = world();
        let mut stack = UndoStack::default();
        stack.undo(&mut world);
        stack.redo(&mut world);
        assert_eq!(count(&world), 0);
    }

    #[test]
    fn undo_and_redo_return_to_the_edits_mode() {
        let mut world = world();
        let mut stack = UndoStack::default();
        edit(&mut world, &mut stack, EditorMode::Terrain, 1);
        let mode = |world: &World| world.resource::<EditorModeSettings>().editor_mode;

        world.resource_mut::<EditorModeSettings>().editor_mode = EditorMode::Scenery;
        stack.undo(&mut world);
        assert_eq!(mode(&world), EditorMode::Terrain);

        world.resource_mut::<EditorModeSettings>().editor_mode = EditorMode::Catalogs;
        stack.redo(&mut world);
        assert_eq!(mode(&world), EditorMode::Terrain);
    }
}
