use bevy::prelude::*;
use bevy_quill::prelude::*;
use bevy_quill_obsidian::focus::TabGroup;

use crate::view::HudCamera;

pub fn setup_editor_view(mut commands: Commands, q_camera: Query<Entity, With<HudCamera>>) {
    let camera = q_camera.get_single().expect("HudCamera not found");
    commands.spawn(EditorView(camera).to_root());
}

#[derive(Clone, PartialEq)]
struct EditorView(Entity);

impl ViewTemplate for EditorView {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let camera = self.0;

        // Needed to ensure popup menus and dialogs render on the correct camera.
        cx.insert(TargetCamera(camera));

        Element::<NodeBundle>::new()
            .insert((TabGroup::default(), TargetCamera(camera)))
            .children("Editor")
    }
}
