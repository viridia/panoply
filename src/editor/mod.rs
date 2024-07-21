use bevy::prelude::*;
mod camera;
mod ui;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, ui::setup_editor_view)
            .add_systems(Update, camera::camera_controller);
        // app.add_startup_system(camera_controller.system());
    }
}
