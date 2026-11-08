//! The game UI's fonts: dialogs, menus, signs. The editor uses feathers' own.

use bevy::{
    asset::{AssetServer, Handle},
    prelude::{FromWorld, Resource, World},
    text::Font,
};

/// The family name of the game UI's font, for `FontSource::Family`. Text naming it renders once the
/// font has loaded.
pub const GAME_UI_FONT: &str = "Macondo Swash Caps";

/// The family name of the font that names keys in prompts, plainer than [`GAME_UI_FONT`] so that a
/// key reads at a glance.
pub const KEY_PROMPT_FONT: &str = "Rubik";

/// Holds the game UI's fonts, so that they stay loaded and their families stay registered.
#[derive(Resource)]
pub(crate) struct GameFonts {
    _ui: Handle<Font>,
    _key_prompt: Handle<Font>,
}

impl FromWorld for GameFonts {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            _ui: assets.load("fonts/MacondoSwashCaps/MacondoSwashCaps-Regular.ttf"),
            _key_prompt: assets.load("fonts/Rubik/Rubik-VariableFont_wght.ttf"),
        }
    }
}
