use crate::{
    editor::{events::ModifyTerrainMapEvent, EditorMode, SelectedParcel},
    terrain::{Parcel, ShapeRef},
};
use bevy::{prelude::*, ui};
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, size::Size, RoundedCorners};

use super::{controls::ContourChooser, tool_terrain_edit};

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("terrain_tool"))]
pub(crate) enum TerrainTool {
    #[default]
    RaiseDraw,
    RaiseRect,
    LowerDraw,
    LowerRect,
    FlattenDraw,
    FlattenRect,
    DrawTrees,
    DrawShrubs,
    DrawHerbs,
    EraseFlora,
}

pub(crate) struct EditTerrainPlugin;

impl Plugin for EditTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(TerrainTool::default())
            .enable_state_scoped_entities::<TerrainTool>()
            .register_type::<State<TerrainTool>>()
            .register_type::<NextState<TerrainTool>>()
            .add_systems(OnEnter(EditorMode::Terrain), tool_terrain_edit::enter)
            .add_systems(OnExit(EditorMode::Terrain), tool_terrain_edit::exit)
            .add_systems(
                Update,
                tool_terrain_edit::hover.run_if(in_state(EditorMode::Terrain)),
            );
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeTerrainControls;

impl ViewTemplate for EditModeTerrainControls {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<TerrainTool>>().get();

        Element::<NodeBundle>::new().style(style_panel).children((
            ToolPalette::new().columns(3).size(Size::Xl).children((
                ToolIconButton::new("editor/icons/raise-draw.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .corners(RoundedCorners::TopLeft)
                    .selected(st == TerrainTool::RaiseDraw)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::RaiseDraw);
                        }),
                    ),
                ToolIconButton::new("editor/icons/lower-draw.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::LowerDraw)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::LowerDraw);
                        }),
                    ),
                ToolIconButton::new("editor/icons/flatten-draw.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .corners(RoundedCorners::TopRight)
                    .selected(st == TerrainTool::FlattenDraw)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::FlattenDraw);
                        }),
                    ),
                ToolIconButton::new("editor/icons/raise-rect.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::RaiseRect)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::RaiseRect);
                        }),
                    ),
                ToolIconButton::new("editor/icons/lower-rect.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::LowerRect)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::LowerRect);
                        }),
                    ),
                ToolIconButton::new("editor/icons/flatten-rect.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::FlattenRect)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::FlattenRect);
                        }),
                    ),
                ToolIconButton::new("editor/icons/pine.png")
                    .size(Vec2::new(24., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::DrawTrees)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::DrawTrees);
                        }),
                    ),
                ToolIconButton::new("editor/icons/shrub.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::DrawShrubs)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::DrawShrubs);
                        }),
                    ),
                ToolIconButton::new("editor/icons/herb.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .selected(st == TerrainTool::DrawHerbs)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::DrawHerbs);
                        }),
                    ),
                ToolIconButton::new("editor/icons/chop.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .corners(RoundedCorners::BottomLeft)
                    .selected(st == TerrainTool::EraseFlora)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<TerrainTool>>| {
                            mode.set(TerrainTool::EraseFlora);
                        }),
                    ),
                ToolIconButton::new("editor/icons/rotate-ccw.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .on_click(cx.create_callback(
                        |mut commands: Commands,
                         q_parcels: Query<&Parcel>,
                         r_selected_parcel: Res<SelectedParcel>| {
                            let Some(parcel_id) = r_selected_parcel.0 else {
                                return;
                            };
                            let Ok(parcel) = q_parcels.get(parcel_id) else {
                                return;
                            };
                            commands.trigger(ModifyTerrainMapEvent {
                                realm: parcel.realm,
                                coords: parcel.coords,
                                shape: ShapeRef {
                                    shape: parcel.center_shape().shape,
                                    rotation: (parcel.center_shape().rotation + 3) & 3,
                                },
                            });
                        },
                    )),
                ToolIconButton::new("editor/icons/rotate-cw.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .corners(RoundedCorners::BottomRight)
                    .on_click(cx.create_callback(
                        |mut commands: Commands,
                         q_parcels: Query<&Parcel>,
                         r_selected_parcel: Res<SelectedParcel>| {
                            let Some(parcel_id) = r_selected_parcel.0 else {
                                return;
                            };
                            let Ok(parcel) = q_parcels.get(parcel_id) else {
                                return;
                            };
                            commands.trigger(ModifyTerrainMapEvent {
                                realm: parcel.realm,
                                coords: parcel.coords,
                                shape: ShapeRef {
                                    shape: parcel.center_shape().shape,
                                    rotation: (parcel.center_shape().rotation + 1) & 3,
                                },
                            });
                        },
                    )),
            )),
            ContourChooser::new().style(|sb: &mut StyleBuilder| {
                sb.grid_row_span(3);
            }),
            ListView::new(),
            ListView::new(),
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
            ui::RepeatedGridTrack::flex(1, 1.),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .gap(8)
        .flex_grow(1.);
}
