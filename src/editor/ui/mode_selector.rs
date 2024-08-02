use crate::editor::EditorMode;
use bevy::prelude::*;
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, RoundedCorners};

use super::{
    mode_meta::EditModeMetadataControls, mode_play::EditModePlayControls,
    mode_realm::EditModeRealmControls, mode_scenery::EditModeSceneryControls,
    mode_terrain::EditModeTerrainControls,
};

#[derive(Clone, PartialEq)]
pub(crate) struct ModeSelector;

impl ViewTemplate for ModeSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<EditorMode>>().get();

        ToolPalette::new().size(Size::Xl).columns(5).children((
            ToolIconButton::new("editor/icons/world.png")
                .corners(RoundedCorners::Left)
                .no_tint(true)
                .size(Vec2::new(32., 24.))
                .selected(st == EditorMode::Realm)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorMode>>| {
                        mode.set(EditorMode::Realm);
                    }),
                ),
            ToolIconButton::new("editor/icons/terrain.png")
                .no_tint(true)
                .size(Vec2::new(32., 24.))
                .selected(st == EditorMode::Terrain)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorMode>>| {
                        mode.set(EditorMode::Terrain);
                    }),
                ),
            ToolIconButton::new("editor/icons/building.png")
                .no_tint(true)
                .size(Vec2::new(32., 24.))
                .selected(st == EditorMode::Scenery)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorMode>>| {
                        mode.set(EditorMode::Scenery);
                    }),
                ),
            ToolIconButton::new("editor/icons/quest.png")
                .no_tint(true)
                .size(Vec2::new(30., 24.))
                .selected(st == EditorMode::Meta)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorMode>>| {
                        mode.set(EditorMode::Meta);
                    }),
                ),
            ToolIconButton::new("editor/icons/play.png")
                .corners(RoundedCorners::Right)
                .no_tint(true)
                .size(Vec2::new(28., 24.))
                .selected(st == EditorMode::Play)
                .on_click(
                    cx.create_callback(|mut mode: ResMut<NextState<EditorMode>>| {
                        mode.set(EditorMode::Play);
                    }),
                ),
        ))
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct EditorModalControls;

impl ViewTemplate for EditorModalControls {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<EditorMode>>().get();

        Switch::new(st)
            .case(EditorMode::Realm, EditModeRealmControls)
            .case(EditorMode::Terrain, EditModeTerrainControls)
            .case(EditorMode::Scenery, EditModeSceneryControls)
            .case(EditorMode::Meta, EditModeMetadataControls)
            .case(EditorMode::Play, EditModePlayControls)
    }
}
