use bevy::prelude::*;
use bevy_mod_preferences::{Preferences, ReflectPreferences};

mod camera;
mod ui;

pub struct EditorPlugin;

#[derive(Resource)]
pub struct EditorSidebarWidth(pub f32);

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
enum EditorState {
    #[default]
    World,
    Terrain,
    Scenery,
    Meta,
    Play,
}

#[derive(Resource, Default, Reflect, PartialEq)]
#[reflect(Default, Preferences)]
struct EditorPrefs {
    pub sidebar_width: f32,
    pub mode: EditorState,
}

impl Preferences for EditorPrefs {}

impl Default for EditorSidebarWidth {
    fn default() -> Self {
        Self(300.0)
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(EditorState::Meta)
            .init_resource::<EditorSidebarWidth>()
            .init_resource::<EditorPrefs>()
            .insert_state(ui::quick_nav::QuickNavOpen::default())
            .add_systems(PostStartup, ui::setup_editor_view)
            .add_systems(Update, camera::camera_controller);
    }
}
