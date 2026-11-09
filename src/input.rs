use bevy::{
    input::mouse::MouseScrollUnit, picking::events::PointerScroll, prelude::*,
    settings::SaveSettingsSync,
};
use bevy_action_map::prelude::*;
use panoply_actors::{PlayerControl, PlayerTurn};
use panoply_core::{GameMode, GameViewNode, QuitGate, QuitRequest, Viewpoint};

const CAMERA_SPEED: f32 = 10.;
const CAMERA_ROTATION_SPEED: f32 = 1.5;
/// How fast the turn speed approaches the keys', per second (exponential).
const TURN_EASE_RATE: f32 = 16.;
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

/// The free camera: keyboard control of the viewpoint, used while editing.
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
                // In play the same keys drive the player's actor (`PlayerControl`).
                context.active_in_state(GameMode::Edit);
                context.bind::<MoveForward>(AxisButtons::new(KeyCode::ArrowDown, KeyCode::ArrowUp));
                context.bind::<MoveForward>(AxisButtons::new(KeyCode::KeyS, KeyCode::KeyW));
                context.bind::<Turn>(AxisButtons::left_right());
                context.bind::<Turn>(AxisButtons::new(KeyCode::KeyA, KeyCode::KeyD));
                // Shift+Arrow is more specific than Arrow, so it takes over from Turn.
                context
                    .bind::<Strafe>(AxisButtons::left_right())
                    .with(ModifierKey::Shift);
                context
                    .bind::<Strafe>(AxisButtons::new(KeyCode::KeyA, KeyCode::KeyD))
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
            .add_systems(Update, (move_viewpoint, quit, perform_quit))
            .add_observer(scroll_rotate_viewpoint);
    }
}

/// Moves the free camera, and turns the view in either mode: in play the player's keys turn it
/// (R29), and their actor turns to match only as it walks.
fn move_viewpoint(
    input: ContextActions<ViewpointControl>,
    player: ActionsQuery<PlayerControl>,
    mut viewpoint: ResMut<Viewpoint>,
    time: Res<Time>,
    mut turn_speed: Local<f32>,
) {
    let delta = time.delta_secs();
    let fast = if input.value::<Fast>() {
        FAST_MULTIPLIER
    } else {
        1.
    };
    let forward = input.value::<MoveForward>();
    // The context not in its mode is inactive and reads 0.
    let turn = input.value::<Turn>()
        + player
            .iter()
            .map(|(_, state)| state.value::<PlayerTurn>())
            .sum::<f32>();
    let strafe = input.value::<Strafe>();

    // Turning eases in and out, as faery's camera did.
    let wanted = -turn * CAMERA_ROTATION_SPEED;
    *turn_speed = wanted + (*turn_speed - wanted) * (-TURN_EASE_RATE * delta).exp();
    if turn_speed.abs() > 1e-4 {
        viewpoint.rotate(*turn_speed * delta);
    }
    if forward != 0. || strafe != 0. {
        viewpoint.follow = None;
        viewpoint.move_local(
            forward * CAMERA_SPEED * delta * fast,
            -strafe * CAMERA_SPEED * delta,
        );
    }
}

/// Horizontal trackpad scroll over the game's view rotates the viewpoint; a scroll over UI goes to
/// the UI. Every scroll bubbles up to the window, so this acts there, once, on scrolls that started
/// outside the UI (on the window or in the world) or within a [`GameViewNode`].
fn scroll_rotate_viewpoint(
    scroll: On<PointerScroll>,
    windows: Query<(), With<Window>>,
    ui_nodes: Query<(), With<Node>>,
    game_views: Query<(), With<GameViewNode>>,
    parents: Query<&ChildOf>,
    mut viewpoint: ResMut<Viewpoint>,
) {
    let at = scroll.event_target();
    if !windows.contains(at) || scroll.unit != MouseScrollUnit::Pixel {
        return;
    }
    let origin = scroll.original_event_target();
    let over_view = !ui_nodes.contains(origin)
        || std::iter::once(origin)
            .chain(parents.iter_ancestors(origin))
            .any(|e| game_views.contains(e));
    if over_view {
        viewpoint.rotate(scroll.x * -SCROLL_ROTATION_SPEED);
    }
}

fn quit(input: ContextActions<AppControl>, mut requests: MessageWriter<QuitRequest>) {
    if input.fired::<Quit>() {
        requests.write(QuitRequest);
    }
}

/// Saves settings and exits on a [`QuitRequest`], unless the [`QuitGate`] is held.
fn perform_quit(
    mut requests: MessageReader<QuitRequest>,
    gate: Res<QuitGate>,
    mut commands: Commands,
) {
    if requests.read().count() > 0 && !gate.held {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
    }
}
