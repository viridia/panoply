use bevy::{
    prelude::*,
    window::{WindowCloseRequested, WindowMode},
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
extern crate directories;
use directories::ProjectDirs;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct WindowSettings {
    pub fullscreen: bool,
    pub position: IVec2,
    pub size: UVec2,
}

#[derive(Resource, Default, Serialize, Deserialize, Debug)]
pub struct UserSettings {
    pub window: WindowSettings,
}

/// System which keeps the window settings up to date when the user resizes or moves the window.
/// Also writes the settings when the window is closed.
pub fn update_window_settings(
    mut move_events: EventReader<WindowMoved>,
    mut close_requested_events: EventReader<WindowCloseRequested>,
    mut settings: ResMut<UserSettings>,
    windows: Query<&mut Window>,
) {
    for event in move_events.iter() {
        settings.window.position = event.position;
        // println!("Window moved: {} {}", event.position.x, event.position.y);
    }

    for _ in close_requested_events.iter() {
        let window = windows.single();
        settings.window.size = UVec2::new(
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        );
        settings.window.fullscreen = window.mode != WindowMode::Windowed;
        save_user_settings(&settings);
        // println!("Window closed requested");
    }
}

pub fn load_user_settings() -> Option<UserSettings> {
    if let Some(proj_dirs) = ProjectDirs::from("org", "viridia", "bevy-game") {
        let config_path = proj_dirs.config_dir().join("settings.json");
        if config_path.is_file() {
            let text = std::fs::read_to_string(&config_path).unwrap();
            return Some(serde_json::from_str::<UserSettings>(&text).unwrap());
        }
    }
    None
}

pub fn save_user_settings(prefs: &UserSettings) {
    if let Some(proj_dirs) = ProjectDirs::from("org", "viridia", "bevy-game") {
        std::fs::create_dir_all(proj_dirs.config_dir()).expect("Failed to create config dir.");
        std::fs::write(
            proj_dirs.config_dir().join("settings.json"),
            serde_json::to_string(&prefs).unwrap(),
        )
        .unwrap();
    }
}
