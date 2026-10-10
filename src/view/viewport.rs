use bevy::{camera::Viewport, prelude::*, reflect::Reflect, window::PrimaryWindow};
use panoply_core::PrimaryCamera;

// use crate::guise::{Controller, ViewElement};

const DEFAULT_FOV: f32 = 0.69; // 40 degrees

/// Used to create margins around the viewport so that side panels don't overwrite the 3d scene.
/// Reflected so that test plans can shrink the 3D view over BRP.
#[derive(Default, Resource, PartialEq, Reflect)]
#[reflect(Resource)]
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
    let Ok(window) = windows.single() else {
        return;
    };
    let ww = window.resolution.physical_width() as f32;
    let wh = window.resolution.physical_height() as f32;
    let sf = window.resolution.scale_factor();
    let left = (viewport_inset.left * sf).clamp(0., ww);
    let right = (viewport_inset.right * sf).clamp(0., ww);
    let top = (viewport_inset.top * sf).clamp(0., wh);
    let bottom = (viewport_inset.bottom * sf).clamp(0., wh);
    let vw = (ww - left - right).max(1.);
    let vh = (wh - top - bottom).max(1.);

    let Ok((mut camera, mut projection)) = camera_query.single_mut() else {
        return;
    };
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
#[derive(Reflect, Default, Component, Clone)]
#[reflect(Default)]
#[reflect(Component)]
pub struct ViewportInsetElement;

pub fn update_viewport_inset(
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_viewport: Query<(&ComputedNode, &GlobalTransform), With<ViewportInsetElement>>,
    mut viewport_inset: ResMut<ViewportInset>,
    mut had_panel: Local<bool>,
) {
    let mut inset = ViewportInset::default();
    match q_viewport.single() {
        Ok((node, transform)) => {
            *had_panel = true;
            let position = transform.translation();
            let ui_position = position.truncate();
            let extents = node.size() / 2.0;
            let min = ui_position - extents;
            let max = ui_position + extents;

            let Ok(window) = q_windows.single() else {
                return;
            };
            let ww = window.resolution.physical_width() as f32;
            let wh = window.resolution.physical_height() as f32;
            let sf = window.resolution.scale_factor();

            inset.left = min.x;
            inset.top = min.y;
            inset.right = ww / sf - max.x;
            inset.bottom = wh / sf - max.y;
        }
        Err(_) => {
            if q_viewport.iter().count() > 1 {
                error!("Multiple ViewportInsetControllers!");
            } else if !std::mem::take(&mut *had_panel) {
                // With no panel, leave the inset alone, so that it can be set directly (test
                // plans do, over BRP). It is reset once, when a panel goes away.
                return;
            }
        }
    }

    if inset != *viewport_inset {
        *viewport_inset.as_mut() = inset;
    }
}
