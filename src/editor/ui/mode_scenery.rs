use crate::{
    actors::ACTOR_TYPE,
    editor::EditorMode,
    scenery::{FIXTURE_TYPE, FLOOR_TYPE, WALL_TYPE},
};
use bevy::{prelude::*, ui};
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, RoundedCorners};

use super::controls::ExemplarChooser;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("scenery_tool"))]
pub enum SceneryTool {
    #[default]
    FloorDraw,
    WallDraw,
    FixtureDraw,
    ActorPlacement,
    TerrainFxDraw,
    SceneryEdit,
    EditLayers,
    SceneryRect,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("floor_tool"))]
pub enum FloorTool {
    #[default]
    Move,
    Draw,
    RectM,
    RectL,
    RectXL,
    Beveled,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("wall_snap"))]
pub enum WallSnap {
    #[default]
    Normal,
    Offset,
    Quarter,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SceneryOverlay {
    FloorDraw,
    FloorCreate,
    PlaceWall,
    PlaceFixture,
    PlaceActor,
    DrawTerrainFx,
    Interact,
    RectSelect,
}

impl ComputedStates for SceneryOverlay {
    type SourceStates = (EditorMode, SceneryTool, FloorTool);

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        if sources.0 != EditorMode::Scenery {
            return None;
        }
        match sources.1 {
            SceneryTool::FloorDraw => match sources.2 {
                FloorTool::Move | FloorTool::Draw => Some(SceneryOverlay::FloorDraw),
                FloorTool::RectM | FloorTool::RectL | FloorTool::RectXL | FloorTool::Beveled => {
                    Some(SceneryOverlay::FloorCreate)
                }
            },
            SceneryTool::WallDraw => Some(SceneryOverlay::PlaceWall),
            SceneryTool::FixtureDraw => Some(SceneryOverlay::PlaceFixture),
            SceneryTool::ActorPlacement => Some(SceneryOverlay::PlaceActor),
            SceneryTool::TerrainFxDraw => Some(SceneryOverlay::DrawTerrainFx),
            SceneryTool::SceneryEdit => Some(SceneryOverlay::Interact),
            SceneryTool::SceneryRect => Some(SceneryOverlay::RectSelect),
            _ => None,
        }
    }
}

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
                        .selected(st == SceneryTool::FloorDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::FloorDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/wall-draw.png")
                        .size(Vec2::new(24., 24.))
                        .tint(false)
                        .corners(RoundedCorners::TopRight)
                        .selected(st == SceneryTool::WallDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::WallDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/furnishing-draw.png")
                        .size(Vec2::new(20., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::FixtureDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::FixtureDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/actor.png")
                        .size(Vec2::new(24., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::ActorPlacement)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::ActorPlacement);
                            },
                        )),
                    ToolIconButton::new("editor/icons/road-draw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::TerrainFxDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::TerrainFxDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/machine.png")
                        .size(Vec2::new(28., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::SceneryEdit)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::SceneryEdit);
                            },
                        )),
                    ToolIconButton::new("editor/icons/layers.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::EditLayers)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::EditLayers);
                            },
                        )),
                    ToolIconButton::new("editor/icons/rect-select.png")
                        .size(Vec2::new(28., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::SceneryRect)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::SceneryRect);
                            },
                        )),
                    ToolIconButton::new("editor/icons/rotate-ccw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::BottomLeft),
                    ToolIconButton::new("editor/icons/rotate-cw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::BottomRight),
                )),
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
            Element::<NodeBundle>::new()
                .style(style_chooser_panel)
                .children((Switch::new(st)
                    .case(
                        SceneryTool::FloorDraw,
                        (
                            FloorToolSelector,
                            ExemplarChooser {
                                selected: None,
                                instance_type: FLOOR_TYPE,
                                filter: "".to_string(),
                                style: style_exemplar_chooser.into_handle(),
                            },
                        ),
                    )
                    .case(
                        SceneryTool::WallDraw,
                        (
                            WallSnapSelector,
                            ExemplarChooser {
                                selected: None,
                                instance_type: WALL_TYPE,
                                filter: "".to_string(),
                                style: style_exemplar_chooser.into_handle(),
                            },
                        ),
                    )
                    .case(
                        SceneryTool::FixtureDraw,
                        ExemplarChooser {
                            selected: None,
                            instance_type: FIXTURE_TYPE,
                            filter: "".to_string(),
                            style: style_exemplar_chooser.into_handle(),
                        },
                    )
                    .case(
                        SceneryTool::ActorPlacement,
                        ExemplarChooser {
                            selected: None,
                            instance_type: ACTOR_TYPE,
                            filter: "".to_string(),
                            style: style_exemplar_chooser.into_handle(),
                        },
                    )
                    .fallback(()),)),
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

fn style_chooser_panel(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Flex)
        .flex_direction(ui::FlexDirection::Column)
        .align_items(ui::AlignItems::Stretch)
        .gap(8)
        .min_height(0)
        .grid_row_start(1)
        .grid_row_span(3)
        .grid_column_start(2)
        .grid_column_span(1);
}

fn style_exemplar_chooser(ss: &mut StyleBuilder) {
    ss.min_height(0).flex_grow(1.);
}

#[derive(Clone, PartialEq)]
pub(crate) struct FloorToolSelector;

impl ViewTemplate for FloorToolSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<FloorTool>>().get();

        ToolPalette::new()
            .columns(6)
            .size(Size::Lg)
            .style(|sb: &mut StyleBuilder| {
                sb.align_self(ui::AlignSelf::Start);
            })
            .children((
                ToolIconButton::new("editor/icons/pointer.png")
                    .size(Vec2::new(13., 16.))
                    .corners(RoundedCorners::Left)
                    .selected(st == FloorTool::Move)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::Move);
                        }),
                    ),
                ToolIconButton::new("editor/icons/pencil.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::Draw)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::Draw);
                        }),
                    ),
                ToolIconButton::new("editor/icons/tile1.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::RectM)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::RectM);
                        }),
                    ),
                ToolIconButton::new("editor/icons/tile2.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::RectL)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::RectL);
                        }),
                    ),
                ToolIconButton::new("editor/icons/tile3.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::RectXL)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::RectXL);
                        }),
                    ),
                ToolIconButton::new("editor/icons/octagon.png")
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Right)
                    .selected(st == FloorTool::Beveled)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::Beveled);
                        }),
                    ),
            ))
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct WallSnapSelector;

impl ViewTemplate for WallSnapSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<WallSnap>>().get();

        ToolPalette::new()
            .columns(3)
            // .size(Size::Xl)
            .style(|sb: &mut StyleBuilder| {
                sb.align_self(ui::AlignSelf::Start);
            })
            .children((
                ToolIconButton::new("editor/icons/grid-normal.png")
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Left)
                    .selected(st == WallSnap::Normal)
                    .on_click(cx.create_callback(|mut mode: ResMut<NextState<WallSnap>>| {
                        mode.set(WallSnap::Normal);
                    })),
                ToolIconButton::new("editor/icons/grid-offset.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == WallSnap::Offset)
                    .on_click(cx.create_callback(|mut mode: ResMut<NextState<WallSnap>>| {
                        mode.set(WallSnap::Offset);
                    })),
                ToolIconButton::new("editor/icons/grid-fine.png")
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Right)
                    .selected(st == WallSnap::Quarter)
                    .on_click(cx.create_callback(|mut mode: ResMut<NextState<WallSnap>>| {
                        mode.set(WallSnap::Quarter);
                    })),
            ))
    }
}
