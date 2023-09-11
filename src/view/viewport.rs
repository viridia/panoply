use bevy::{prelude::*, reflect::Reflect, render::camera::Viewport, window::PrimaryWindow};

use crate::guise::{Controller, ViewElement};

use super::PrimaryCamera;

const DEFAULT_FOV: f32 = 0.69; // 40 degrees

/// Used to create margins around the viewport so that side panels don't overwrite the 3d scene.
#[derive(Default, Resource, PartialEq)]
pub struct ViewportInset {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// Update the camera viewport and fov properties based on the window size and the viewport
/// margins.
pub fn update_camera_viewport(
    viewport_inset: Res<ViewportInset>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Camera, &mut Projection), With<PrimaryCamera>>,
) {
    let window = windows.single();
    let ww = window.resolution.physical_width() as f32;
    let wh = window.resolution.physical_height() as f32;
    let sf = window.resolution.scale_factor() as f32;
    let left = viewport_inset.left * sf;
    let right = viewport_inset.right * sf;
    let top = viewport_inset.top * sf;
    let bottom = viewport_inset.bottom * sf;
    let vw = (ww - left - right).max(1.);
    let vh = (wh - top - bottom).max(1.);

    let (mut camera, mut projection) = camera_query.single_mut();
    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(left as u32, top as u32),
        physical_size: UVec2::new(vw as u32, vh as u32),
        ..default()
    });

    if let Projection::Perspective(ref mut perspective) = *projection {
        let aspect = vw / vh;
        perspective.aspect_ratio = aspect;
        perspective.fov = f32::min(DEFAULT_FOV, DEFAULT_FOV * 2. / aspect);
        perspective.near = 0.5;
        perspective.far = 100.;
    }
}

/// Controller which updates the margins of the main 3D view so that it fits within the
/// designated 2D panel.
#[derive(Reflect, Default, Component)]
#[reflect(Default)]
#[reflect(Component)]
pub struct ViewportInsetController {}

impl Controller for ViewportInsetController {}

pub fn update_viewport_inset(
    windows: Query<&Window, With<PrimaryWindow>>,
    query: Query<(&ViewElement, &Node, &GlobalTransform), With<ViewportInsetController>>,
    mut viewport_inset: ResMut<ViewportInset>,
) {
    let mut inset = ViewportInset::default();
    match query.get_single() {
        Ok((_, node, transform)) => {
            let position = transform.translation();
            let ui_position = position.truncate();
            let extents = node.size() / 2.0;
            let min = ui_position - extents;
            let max = ui_position + extents;

            let window = windows.single();
            let ww = window.resolution.physical_width() as f32;
            let wh = window.resolution.physical_height() as f32;
            let sf = window.resolution.scale_factor() as f32;

            inset.left = min.x;
            inset.top = min.y;
            inset.right = ww / sf - max.x;
            inset.bottom = wh / sf - max.y;
        }
        Err(_) => {
            if query.iter().count() > 1 {
                error!("Multiple ViewportInsetControllers!");
            }
        }
    }

    if inset != *viewport_inset {
        *viewport_inset.as_mut() = inset;
    }
}
