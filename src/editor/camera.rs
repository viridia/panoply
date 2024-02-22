use bevy::{input::mouse::MouseWheel, prelude::*};

use crate::view::{PrimaryCamera, Viewpoint};

const CAMERA_SPEED: f32 = 10.;
const CAMERA_ROTATION_SPEED: f32 = 1.5;

fn movement(flag: bool) -> f32 {
    if flag {
        1.
    } else {
        0.
    }
}

pub fn camera_controller(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_events: EventReader<MouseWheel>,
    mut viewpoint: ResMut<Viewpoint>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<PrimaryCamera>>,
) {
    let mut transform = query.single_mut();

    let strafe =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);
    let fast = if strafe { 3. } else { 1. };
    let left = keyboard_input.pressed(KeyCode::ArrowLeft);
    let right = keyboard_input.pressed(KeyCode::ArrowRight);
    let up = keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW);
    let down = keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS);

    use bevy::input::mouse::MouseScrollUnit;
    for ev in scroll_events.read() {
        match ev.unit {
            MouseScrollUnit::Line => {}
            MouseScrollUnit::Pixel => {
                viewpoint.rotate(ev.x * -0.002);
            }
        }
    }

    viewpoint.rotate(
        (movement(left && !strafe) - movement(right && !strafe))
            * CAMERA_ROTATION_SPEED
            * time.delta_seconds(),
    );

    viewpoint.move_local(
        (movement(up) - movement(down)) * CAMERA_SPEED * time.delta_seconds() * fast,
        (movement(left && strafe) - movement(right && strafe))
            * CAMERA_SPEED
            * time.delta_seconds(),
    );

    viewpoint.get_camera_transform(&mut transform);
}
