use bevy::{ecs::world::Command, prelude::*};

pub trait UndoEntry: Send + Sync + 'static {
    fn label(&self) -> &str;
    fn undo(&self, world: &mut World) -> Box<dyn RedoEntry>;
}

pub trait RedoEntry: Send + Sync + 'static {
    fn label(&self) -> &str;
    fn redo(&self, world: &mut World) -> Box<dyn UndoEntry>;
}

/// Resource that manages the undo / redo stack.
#[derive(Default, Resource)]
pub struct UndoStack {
    undo_stack: Vec<Box<dyn UndoEntry>>,
    redo_stack: Vec<Box<dyn RedoEntry>>,
}

impl UndoStack {
    const MAX_UNDO: usize = 100;

    pub fn push(&mut self, entry: impl UndoEntry) {
        self.undo_stack.push(Box::new(entry));
        let len = self.undo_stack.len();
        if len > Self::MAX_UNDO {
            self.undo_stack.drain(0..(len - Self::MAX_UNDO));
        }
        self.redo_stack.clear();
    }

    pub fn next_undo_label(&self) -> Option<&str> {
        self.undo_stack.last().map(|entry| entry.label())
    }

    pub fn next_redo_label(&self) -> Option<&str> {
        self.redo_stack.last().map(|entry| entry.label())
    }

    pub fn undo(&mut self, world: &mut World) {
        if let Some(entity) = self.undo_stack.pop() {
            self.redo_stack.push(entity.undo(world));
        }
    }

    pub fn redo(&mut self, world: &mut World) {
        if let Some(entity) = self.redo_stack.pop() {
            self.undo_stack.push(entity.redo(world));
        }
    }
}

pub struct UndoCommand;

impl Command for UndoCommand {
    fn apply(self, world: &mut World) {
        world.resource_scope(|world, mut stack: Mut<UndoStack>| stack.undo(world));
    }
}

pub struct RedoCommand;

impl Command for RedoCommand {
    fn apply(self, world: &mut World) {
        world.resource_scope(|world, mut stack: Mut<UndoStack>| stack.redo(world));
    }
}
