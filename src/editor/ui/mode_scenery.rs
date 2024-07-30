use crate::editor::SceneryTool;
use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, RoundedCorners};

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeSceneryControls;

impl ViewTemplate for EditModeSceneryControls {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<SceneryTool>>().get();

        Element::<NodeBundle>::new().style(style_panel).children((
            ToolPalette::new()
                .columns(2)
                .size(Size::Xl)
                .style(|sb: &mut StyleBuilder| {
                    sb.grid_column_start(1).grid_row_start(1);
                })
                .children((
                    ToolIconButton::new("editor/icons/floor-draw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::TopLeft)
                        .selected(st == SceneryTool::FloorDraw),
                    ToolIconButton::new("editor/icons/wall-draw.png")
                        .size(Vec2::new(24., 24.))
                        .tint(false)
                        .corners(RoundedCorners::TopRight)
                        .selected(st == SceneryTool::WallDraw),
                    ToolIconButton::new("editor/icons/furnishing-draw.png")
                        .size(Vec2::new(20., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::FixtureDraw),
                    ToolIconButton::new("editor/icons/actor.png")
                        .size(Vec2::new(24., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::ActorPlacement),
                    ToolIconButton::new("editor/icons/road-draw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::TerrainFxDraw),
                    ToolIconButton::new("editor/icons/machine.png")
                        .size(Vec2::new(28., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::SceneryEdit),
                    ToolIconButton::new("editor/icons/layers.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::EditLayers),
                    ToolIconButton::new("editor/icons/rect-select.png")
                        .size(Vec2::new(28., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::SceneryRect),
                    ToolIconButton::new("editor/icons/rotate-ccw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::BottomLeft),
                    ToolIconButton::new("editor/icons/rotate-cw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::BottomRight),
                )),
            Checkbox::new().label("label"),
            ToolPalette::new()
                .columns(3)
                .style(|sb: &mut StyleBuilder| {
                    sb.grid_column_start(1).grid_row_start(2);
                })
                .children((
                    ToolButton::new()
                        .children("Cut")
                        .corners(RoundedCorners::Left)
                        .selected(st == SceneryTool::FloorDraw),
                    ToolIconButton::new("editor/icons/rotate-ccw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false),
                    ToolIconButton::new("editor/icons/rotate-cw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::Right),
                )),
            ListView::new().style(|sb: &mut StyleBuilder| {
                sb.grid_row_start(3).grid_row_end(4).min_height(48);
            }),
        ))
    }
}

fn style_panel(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Grid)
        .grid_template_columns(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .grid_template_rows(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .gap(8)
        .flex_grow(1.);
}
