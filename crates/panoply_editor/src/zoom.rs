//! Camera zoom: in the game the camera distance is fixed.

use std::time::Duration;

use bevy::{
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use bevy_action_map::prelude::*;
use panoply_core::Viewpoint;

/// Camera distance at zoom level 0; each level doubles it.
const ZOOM_BASE_DISTANCE: f32 = 20.;
const ZOOM_MIN_LEVEL: f32 = 0.;
const ZOOM_MAX_LEVEL: f32 = 2.;
const ZOOM_STEP: f32 = 0.5;
const ZOOM_SECONDS: f32 = 0.2;

/// Camera zoom, saved between runs. Distance is `ZOOM_BASE_DISTANCE * 2^level`.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "view")]
pub struct ZoomSettings {
    pub level: f32,
}

impl Default for ZoomSettings {
    fn default() -> Self {
        Self { level: 1.5 }
    }
}

impl ZoomSettings {
    fn distance(&self) -> f32 {
        ZOOM_BASE_DISTANCE * 2f32.powf(self.level)
    }
}

/// Zoom the camera in by one step per press.
#[derive(InputAction)]
#[action(path = "editor.zoom_in", output = bool, intent = Button)]
pub struct ZoomIn;

/// Zoom the camera out by one step per press.
#[derive(InputAction)]
#[action(path = "editor.zoom_out", output = bool, intent = Button)]
pub struct ZoomOut;

/// Editor camera controls.
#[derive(InputContext)]
#[context(path = "editor.camera", tick = Render)]
pub struct EditorCameraControl;

pub struct EditorZoomPlugin;

impl Plugin for EditorZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_context::<EditorCameraControl>(|context| {
            context.bind::<ZoomIn>(KeyCode::Equal);
            context.bind::<ZoomOut>(KeyCode::Minus);
        })
        .add_systems(
            Startup,
            (
                |mut commands: Commands| {
                    commands.spawn((Name::new("EditorCameraControl"), EditorCameraControl));
                },
                apply_zoom_settings,
            ),
        )
        .add_systems(Update, zoom_viewpoint);
    }
}

fn apply_zoom_settings(zoom: Res<ZoomSettings>, mut viewpoint: ResMut<Viewpoint>) {
    viewpoint.set_camera_distance(zoom.distance(), 0.);
}

fn zoom_viewpoint(
    input: ContextActions<EditorCameraControl>,
    mut zoom: ResMut<ZoomSettings>,
    mut viewpoint: ResMut<Viewpoint>,
    mut commands: Commands,
) {
    // `fired` is true only on the tick a key goes down, so each press is one step. Zooming
    // in brings the camera closer, so it lowers the level.
    let step = match (input.fired::<ZoomIn>(), input.fired::<ZoomOut>()) {
        (true, false) => -ZOOM_STEP,
        (false, true) => ZOOM_STEP,
        _ => return,
    };
    let level = (zoom.level + step).clamp(ZOOM_MIN_LEVEL, ZOOM_MAX_LEVEL);
    if zoom.set_if_neq(ZoomSettings { level }) {
        viewpoint.set_camera_distance(zoom.distance(), ZOOM_SECONDS);
        commands.queue(SaveSettingsDeferred(Duration::from_secs_f32(0.5)));
    }
}
