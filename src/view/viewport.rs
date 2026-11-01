use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use panoply_core::{PrimaryCamera, ViewportInset};

const DEFAULT_FOV: f32 = 0.69; // 40 degrees

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
