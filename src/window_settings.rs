use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowMode},
};
use bevy_user_prefs::{Preferences, SetPreferencesChanged};

#[derive(Default, Debug)]
pub struct WindowSettings {
    pub fullscreen: bool,
    pub position: IVec2,
    pub size: UVec2,
}

/// System which keeps the window settings up to date when the user resizes or moves the window.
/// Also writes the settings when the window is closed.
pub fn update_window_settings(
    mut move_events: EventReader<WindowMoved>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
    mut preferences: ResMut<Preferences>,
    mut commands: Commands,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };

    let mut window_moved = false;
    for _event in move_events.read() {
        window_moved = true;
    }

    if window_moved {
        if let Some(app_prefs) = preferences.get_mut("prefs") {
            let mut window_prefs = app_prefs.get_group_mut("window").unwrap();
            window_prefs.set_bool("fullscreen", window.mode != WindowMode::Windowed);
            match window.position {
                WindowPosition::At(pos) => {
                    window_prefs.set_ivec2("position", pos);
                }
                _ => {
                    window_prefs.remove("position");
                }
            }
            window_prefs.set_vec2(
                "size",
                Vec2::new(window.resolution.width(), window.resolution.height()),
            );
            commands.queue(SetPreferencesChanged);
        }
    }
}

pub struct WindowSettingsPlugin;

impl Plugin for WindowSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_window_settings);
    }
}

pub fn load_window_settings(prefs: &mut Preferences, window: &mut Window) {
    if let Some(app_prefs) = prefs.get("prefs") {
        if let Some(window_prefs) = app_prefs.get_group("window") {
            if let Some(fullscreen) = window_prefs.get_bool("fullscreen") {
                window.mode = if fullscreen {
                    WindowMode::SizedFullscreen(MonitorSelection::Current)
                } else {
                    WindowMode::Windowed
                };
            }
            if let Some(pos) = window_prefs.get_ivec2("position") {
                window.position = WindowPosition::new(pos);
            }
            if let Some(size) = window_prefs.get_vec2("size") {
                window.resolution = (size.x, size.y).into();
            }
        }
    }
}
