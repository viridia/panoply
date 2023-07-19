use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};

use super::PrimaryCamera;

const DEFAULT_FOV: f32 = 0.69; // 40 degrees

#[derive(Default, Resource)]
pub struct ViewportInset {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

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
    }
}
