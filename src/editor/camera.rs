use bevy::{input::mouse::MouseWheel, prelude::*};

use crate::view::Viewpoint;

const CAMERA_SPEED: f32 = 10.;
const CAMERA_ROTATION_SPEED: f32 = 1.;

fn movement(flag: bool) -> f32 {
    if flag {
        1.
    } else {
        0.
    }
}

pub fn camera_controller(
    keyboard_input: Res<Input<KeyCode>>,
    mut scroll_events: EventReader<MouseWheel>,
    mut viewpoint: ResMut<Viewpoint>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut transform = query.single_mut();

    let strafe =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);
    let left = keyboard_input.pressed(KeyCode::Left);
    let right = keyboard_input.pressed(KeyCode::Right);
    let up = keyboard_input.pressed(KeyCode::Up) || keyboard_input.pressed(KeyCode::W);
    let down = keyboard_input.pressed(KeyCode::Down) || keyboard_input.pressed(KeyCode::S);

    use bevy::input::mouse::MouseScrollUnit;
    for ev in scroll_events.iter() {
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
        (movement(up) - movement(down)) * CAMERA_SPEED * time.delta_seconds(),
        (movement(right && strafe) - movement(left && strafe))
            * CAMERA_SPEED
            * time.delta_seconds(),
    );

    viewpoint.get_camera_transform(&mut transform);
}
