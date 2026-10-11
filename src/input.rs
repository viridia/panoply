use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    settings::SaveSettingsSync,
};
use bevy_action_map::prelude::*;
use panoply_core::Viewpoint;

const CAMERA_SPEED: f32 = 10.;
const CAMERA_ROTATION_SPEED: f32 = 1.5;
const FAST_MULTIPLIER: f32 = 3.;
const SCROLL_ROTATION_SPEED: f32 = 0.002;

/// Move the viewpoint forward (+) or back (-).
#[derive(InputAction)]
#[action(path = "viewpoint.move_forward", output = f32, intent = Analog1)]
pub struct MoveForward;

/// Rotate the viewpoint left (-) or right (+).
#[derive(InputAction)]
#[action(path = "viewpoint.turn", output = f32, intent = Analog1)]
pub struct Turn;

/// Move the viewpoint sideways, left (-) or right (+).
#[derive(InputAction)]
#[action(path = "viewpoint.strafe", output = f32, intent = Analog1)]
pub struct Strafe;

/// Move forward faster while held.
#[derive(InputAction)]
#[action(path = "viewpoint.fast", output = bool, intent = Button)]
pub struct Fast;

/// Save settings and exit the app.
#[derive(InputAction)]
#[action(path = "app.quit", output = bool, intent = Button)]
pub struct Quit;

/// Keyboard control of the viewpoint, used while editing.
#[derive(InputContext)]
#[context(path = "viewpoint.control", tick = Render)]
pub struct ViewpointControl;

/// App-level commands, always active.
#[derive(InputContext)]
#[context(path = "app", tick = Render)]
pub struct AppControl;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ActionMapPlugin)
            .add_context::<ViewpointControl>(|context| {
                context.bind::<MoveForward>(AxisButtons::new(KeyCode::ArrowDown, KeyCode::ArrowUp));
                context.bind::<MoveForward>(AxisButtons::new(KeyCode::KeyS, KeyCode::KeyW));
                context.bind::<Turn>(AxisButtons::left_right());
                // Shift+Arrow is more specific than Arrow, so it takes over from Turn.
                context
                    .bind::<Strafe>(AxisButtons::left_right())
                    .with(ModifierKey::Shift);
                context.bind::<Fast>(KeyCode::ShiftLeft);
                context.bind::<Fast>(KeyCode::ShiftRight);
            })
            .add_context::<AppControl>(|context| {
                context.bind::<Quit>(KeyCode::Escape);
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("ViewpointControl"), ViewpointControl));
                commands.spawn((Name::new("AppControl"), AppControl));
            })
            .add_systems(Update, (move_viewpoint, scroll_rotate_viewpoint, quit));

        #[cfg(feature = "editor")]
        app.add_plugins(editor_zoom::EditorZoomPlugin);
    }
}

fn move_viewpoint(
    input: ContextActions<ViewpointControl>,
    mut viewpoint: ResMut<Viewpoint>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let fast = if input.value::<Fast>() {
        FAST_MULTIPLIER
    } else {
        1.
    };
    let forward = input.value::<MoveForward>();
    let turn = input.value::<Turn>();
    let strafe = input.value::<Strafe>();

    if turn != 0. {
        viewpoint.rotate(-turn * CAMERA_ROTATION_SPEED * delta);
    }
    if forward != 0. || strafe != 0. {
        viewpoint.move_local(
            forward * CAMERA_SPEED * delta * fast,
            -strafe * CAMERA_SPEED * delta,
        );
    }
}

/// Horizontal trackpad scroll rotates the viewpoint.
// TODO: Observe `PointerScroll` on the game view instead, so scrolls over UI go to the UI
// (ledger X8).
fn scroll_rotate_viewpoint(
    mut scroll_events: MessageReader<MouseWheel>,
    mut viewpoint: ResMut<Viewpoint>,
) {
    for ev in scroll_events.read() {
        if ev.unit == MouseScrollUnit::Pixel {
            viewpoint.rotate(ev.x * -SCROLL_ROTATION_SPEED);
        }
    }
}

fn quit(input: ContextActions<AppControl>, mut commands: Commands) {
    if input.fired::<Quit>() {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
    }
}

/// Camera zoom, an editor-only feature: in the game the camera distance is fixed.
#[cfg(feature = "editor")]
mod editor_zoom {
    use std::time::Duration;

    use bevy::{
        prelude::*,
        settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
    };
    use bevy_action_map::prelude::*;
    use panoply_core::Viewpoint;

    /// Camera distance at zoom level 0; each level doubles it.
    const ZOOM_BASE_DISTANCE: f32 = 20.;
    const ZOOM_MIN_LEVEL: f32 = 0.;
    const ZOOM_MAX_LEVEL: f32 = 2.;
    const ZOOM_STEP: f32 = 0.5;
    const ZOOM_SECONDS: f32 = 0.2;

    /// Camera zoom, saved between runs. Distance is `ZOOM_BASE_DISTANCE * 2^level`.
    #[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug)]
    #[reflect(Resource, SettingsGroup, Default)]
    #[settings_group(group = "view")]
    pub struct ZoomSettings {
        pub level: f32,
    }

    impl Default for ZoomSettings {
        fn default() -> Self {
            Self { level: 1.5 }
        }
    }

    impl ZoomSettings {
        fn distance(&self) -> f32 {
            ZOOM_BASE_DISTANCE * 2f32.powf(self.level)
        }
    }

    /// Zoom the camera in by one step per press.
    #[derive(InputAction)]
    #[action(path = "editor.zoom_in", output = bool, intent = Button)]
    pub struct ZoomIn;

    /// Zoom the camera out by one step per press.
    #[derive(InputAction)]
    #[action(path = "editor.zoom_out", output = bool, intent = Button)]
    pub struct ZoomOut;

    /// Editor camera controls.
    #[derive(InputContext)]
    #[context(path = "editor.camera", tick = Render)]
    pub struct EditorCameraControl;

    pub struct EditorZoomPlugin;

    impl Plugin for EditorZoomPlugin {
        fn build(&self, app: &mut App) {
            app.add_context::<EditorCameraControl>(|context| {
                context.bind::<ZoomIn>(KeyCode::Equal);
                context.bind::<ZoomOut>(KeyCode::Minus);
            })
            .add_systems(
                Startup,
                (
                    |mut commands: Commands| {
                        commands.spawn((Name::new("EditorCameraControl"), EditorCameraControl));
                    },
                    apply_zoom_settings,
                ),
            )
            .add_systems(Update, zoom_viewpoint);
        }
    }

    fn apply_zoom_settings(zoom: Res<ZoomSettings>, mut viewpoint: ResMut<Viewpoint>) {
        viewpoint.set_camera_distance(zoom.distance(), 0.);
    }

    fn zoom_viewpoint(
        input: ContextActions<EditorCameraControl>,
        mut zoom: ResMut<ZoomSettings>,
        mut viewpoint: ResMut<Viewpoint>,
        mut commands: Commands,
    ) {
        // `fired` is true only on the tick a key goes down, so each press is one step. Zooming
        // in brings the camera closer, so it lowers the level.
        let step = match (input.fired::<ZoomIn>(), input.fired::<ZoomOut>()) {
            (true, false) => -ZOOM_STEP,
            (false, true) => ZOOM_STEP,
            _ => return,
        };
        let level = (zoom.level + step).clamp(ZOOM_MIN_LEVEL, ZOOM_MAX_LEVEL);
        if zoom.set_if_neq(ZoomSettings { level }) {
            viewpoint.set_camera_distance(zoom.distance(), ZOOM_SECONDS);
            commands.queue(SaveSettingsDeferred(Duration::from_secs_f32(0.5)));
        }
    }
}
