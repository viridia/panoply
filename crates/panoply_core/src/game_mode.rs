use bevy::prelude::*;

/// Which mode the app is in. Each mode's input is a `bevy_action_map` context active only in that
/// mode (`active_in_state`), so the movement keys drive one thing at a time.
#[derive(States, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameMode {
    /// The movement keys drive the player's actor, and Go To moves the player.
    Play,
    /// The movement keys drive the free camera, and Go To moves the view.
    #[default]
    Edit,
}
