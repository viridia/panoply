use bevy::{prelude::*, render::view::RenderLayers};

use crate::view::PrimaryCamera;
use panoply_core::{Realm, Viewpoint};

const CAMERA_SPEED: f32 = 10.;
const CAMERA_ROTATION_SPEED: f32 = 1.5;

fn movement(flag: bool) -> f32 {
    if flag {
        1.
    } else {
        0.
    }
}

pub fn update_camera_pos(
    viewpoint: ResMut<Viewpoint>,
    mut q_camera: Query<(&mut Transform, &mut RenderLayers), With<PrimaryCamera>>,
    q_realms: Query<&Realm>,
) {
    let (mut transform, mut layers) = q_camera.single_mut();

    // Update the camera transform
    viewpoint.get_camera_transform(&mut transform);

    // Update the camera render layers
    match viewpoint.realm {
        Some(realm) => {
            if let Ok(realm) = q_realms.get(realm) {
                *layers = realm.layer.clone();
            } else {
                *layers = RenderLayers::none();
            }
        }
        None => {
            *layers = RenderLayers::none();
        }
    }
}
