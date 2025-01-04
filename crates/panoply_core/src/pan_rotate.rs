use bevy::{input::mouse::MouseWheel, prelude::*};

use crate::Viewpoint;

const CAMERA_SPEED: f32 = 10.;
const CAMERA_ROTATION_SPEED: f32 = 1.5;

fn movement(flag: bool) -> f32 {
    if flag {
        1.
    } else {
        0.
    }
}

pub fn pan_rotate_viewpoint(
    button_input: Res<ButtonInput<KeyCode>>,
    mut scroll_events: EventReader<MouseWheel>,
    mut viewpoint: ResMut<Viewpoint>,
    time: Res<Time>,
) {
    let strafe =
        button_input.pressed(KeyCode::ShiftLeft) || button_input.pressed(KeyCode::ShiftRight);
    let fast = if strafe { 3. } else { 1. };
    let left = button_input.pressed(KeyCode::ArrowLeft);
    let right = button_input.pressed(KeyCode::ArrowRight);
    let up = button_input.pressed(KeyCode::ArrowUp) || button_input.pressed(KeyCode::KeyW);
    let down = button_input.pressed(KeyCode::ArrowDown) || button_input.pressed(KeyCode::KeyS);
    let delta = time.delta_secs();

    use bevy::input::mouse::MouseScrollUnit;
    for ev in scroll_events.read() {
        match ev.unit {
            MouseScrollUnit::Line => {}
            MouseScrollUnit::Pixel => {
                viewpoint.rotate(ev.x * -0.002);
            }
        }
    }

    if left || right {
        viewpoint.rotate(
            (movement(left && !strafe) - movement(right && !strafe))
                * CAMERA_ROTATION_SPEED
                * delta,
        );
    }

    if up || down || left || right {
        viewpoint.move_local(
            (movement(up) - movement(down)) * CAMERA_SPEED * delta * fast,
            (movement(left && strafe) - movement(right && strafe)) * CAMERA_SPEED * delta,
        );
    }
}
