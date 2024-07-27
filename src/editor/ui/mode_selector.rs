use bevy::prelude::*;
use bevy_quill::prelude::*;
use bevy_quill_obsidian::prelude::*;

use crate::editor::EditorState;

#[derive(Clone, PartialEq)]
pub(crate) struct ModeSelector;

impl ViewTemplate for ModeSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = cx.use_resource::<State<EditorState>>().get().clone();

        ToolPalette::new().size(Size::Xl).columns(5).children((
            ToolIconButton::new("editor/icons/world.png")
                .no_tint(true)
                .size(Vec2::new(32., 24.))
                .selected(st == EditorState::World)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorState>>| {
                        mode.set(EditorState::World);
                    }),
                ),
            ToolIconButton::new("editor/icons/terrain.png")
                .no_tint(true)
                .size(Vec2::new(32., 24.))
                .selected(st == EditorState::Terrain)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorState>>| {
                        mode.set(EditorState::Terrain);
                    }),
                ),
            ToolIconButton::new("editor/icons/building.png")
                .no_tint(true)
                .size(Vec2::new(32., 24.))
                .selected(st == EditorState::Scenery)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorState>>| {
                        mode.set(EditorState::Scenery);
                    }),
                ),
            ToolIconButton::new("editor/icons/quest.png")
                .no_tint(true)
                .size(Vec2::new(30., 24.))
                .selected(st == EditorState::Meta)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorState>>| {
                        mode.set(EditorState::Meta);
                    }),
                ),
            ToolIconButton::new("editor/icons/play.png")
                .no_tint(true)
                .size(Vec2::new(28., 24.))
                .selected(st == EditorState::Play)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorState>>| {
                        mode.set(EditorState::Play);
                    }),
                ),
        ))
    }
}
