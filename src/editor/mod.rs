use bevy::prelude::*;
mod camera;
mod ui;

pub struct EditorPlugin;

#[derive(Resource)]
pub struct EditorSidebarWidth(pub f32);

impl Default for EditorSidebarWidth {
    fn default() -> Self {
        Self(300.0)
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorSidebarWidth>()
            .insert_state(ui::quick_nav::QuickNavOpen::default())
            .add_systems(PostStartup, ui::setup_editor_view)
            .add_systems(Update, camera::camera_controller);
    }
}
