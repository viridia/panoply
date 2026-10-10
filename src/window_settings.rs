use std::time::Duration;

use bevy::{
    ecs::system::NonSendMarker,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SaveSettingsSync, SettingsGroup},
    window::{PrimaryWindow, WindowCloseRequested, WindowMode, WindowResized, WindowResolution},
    winit::WINIT_WINDOWS,
};

/// Settings group which remembers the window position, size and fullscreen state.
#[derive(Resource, SettingsGroup, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "window")]
pub struct WindowSettings {
    pub fullscreen: bool,
    pub position: Option<IVec2>,
    pub size: Option<UVec2>,
}

/// Restores window settings at startup and keeps them up to date afterwards. Must be added
/// after `SettingsPlugin`, which loads the `WindowSettings` resource.
pub struct WindowSettingsPlugin;

impl Plugin for WindowSettingsPlugin {
    fn build(&self, app: &mut App) {
        init_window_settings(app);
        app.add_systems(Update, (update_window_settings, save_on_window_close));
    }
}

/// Copy the loaded settings to the primary window.
fn init_window_settings(app: &mut App) {
    let world = app.world_mut();
    let Some(settings) = world.get_resource::<WindowSettings>().cloned() else {
        return;
    };

    let Ok(mut window) = world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(world)
    else {
        warn!("Primary window not found");
        return;
    };

    if let Some(pos) = settings.position {
        window.position = WindowPosition::new(pos);
    }
    if let Some(size) = settings.size {
        window.resolution = WindowResolution::new(size.x, size.y);
    }
    window.mode = if settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
}

/// System which keeps the window settings up to date when the user resizes or moves the window.
/// Runs on the main thread (`NonSendMarker`) because it reads from `WINIT_WINDOWS`.
fn update_window_settings(
    mut move_events: MessageReader<WindowMoved>,
    mut resize_events: MessageReader<WindowResized>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut settings: ResMut<WindowSettings>,
    mut commands: Commands,
    _main_thread: NonSendMarker,
) {
    // Drain both readers (no short-circuit) so stale messages don't carry over.
    let moved = move_events.read().count() > 0;
    let resized = resize_events.read().count() > 0;
    if !moved && !resized {
        return;
    }
    let Ok((entity, window)) = windows.single() else {
        return;
    };

    let changed = settings.set_if_neq(WindowSettings {
        fullscreen: window.mode != WindowMode::Windowed,
        position: match window.position {
            WindowPosition::At(pos) => Some(pos),
            // A window that has never been moved is still `Centered`/`Automatic`, so ask winit
            // where it actually is. Otherwise a resize-only session would forget the position.
            _ => WINIT_WINDOWS.with_borrow(|winit_windows| {
                let pos = winit_windows.get_window(entity)?.outer_position().ok()?;
                Some(IVec2::new(pos.x, pos.y))
            }),
        },
        size: Some(UVec2::new(
            window.resolution.width() as u32,
            window.resolution.height() as u32,
        )),
    });
    if changed {
        commands.queue(SaveSettingsDeferred(Duration::from_secs_f32(0.5)));
    }
}

/// Save settings and exit when the window is closed. Requires
/// `WindowPlugin::exit_condition` to be `ExitCondition::DontExit`.
fn save_on_window_close(mut close: MessageReader<WindowCloseRequested>, mut commands: Commands) {
    if close.read().next().is_some() {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
    }
}
