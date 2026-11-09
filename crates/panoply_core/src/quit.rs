//! Asking the app to exit, which something can hold back to ask the user first.

use bevy::prelude::{Message, Resource};

/// Asks the app to exit: the app does so unless [`QuitGate::held`].
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct QuitRequest;

/// Whether a [`QuitRequest`] waits on something, such as an answer about unsaved work.
#[derive(Resource, Default, Debug)]
pub struct QuitGate {
    pub held: bool,
}
