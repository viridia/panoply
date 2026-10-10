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
// TODO: Move to bevy_action_map once it supports scroll input (ledger X8).
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
