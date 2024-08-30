use crate::editor::unsaved::{self, UnsavedAssets};
use bevy::prelude::*;
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::IconButton, size::Size};

#[derive(Clone, PartialEq)]
pub(crate) struct SaveButton;

impl ViewTemplate for SaveButton {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let empty = cx.use_resource::<UnsavedAssets>().is_empty();

        IconButton::new("editor/icons/save.png")
            // .style(style_button)
            .size(Size::Xl)
            .disabled(empty)
            .on_click(cx.create_callback(|mut commands: Commands| {
                commands.add(unsaved::SaveCommand);
            }))
    }
}
