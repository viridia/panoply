use crate::{
    editor::{EditorMode, ParcelCursor, SelectedParcel, TerrainTool},
    view::events::{PickAction, PickEvent, PickTarget},
};
use bevy::{prelude::*, ui};
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, size::Size, RoundedCorners};

use super::overlays::{SelectedParcelOverlay, TerrainCursorOverlay};

#[derive(Clone, Component)]
pub struct ParcelOverlay;

pub fn enter(mut commands: Commands) {
    commands.spawn((SelectedParcelOverlay.to_root(), ParcelOverlay));
    commands.spawn((TerrainCursorOverlay.to_root(), ParcelOverlay));
    commands.spawn((
        StateScoped(EditorMode::Terrain),
        Observer::new(on_pick_event),
    ));
}

pub fn exit(mut commands: Commands, q_overlays: Query<Entity, With<ParcelOverlay>>) {
    q_overlays.iter().for_each(|e| commands.entity(e).despawn());
    commands.observe(on_pick_event);
}

pub fn hover(
    r_selected_parcel: Res<SelectedParcel>,
    mut r_parcel_cursor: ResMut<ParcelCursor>,
    r_hover_map: Res<HoverMap>,
    // q_parcels: Query<&Parcel>,
) {
    let mut position: Option<Vec3> = None;
    if r_selected_parcel.0.is_some() {
        if let Some(p) = r_hover_map.get(&PointerId::Mouse) {
            for (_, hit_data) in p.iter() {
                if hit_data.position.is_some() {
                    position = hit_data.position;
                    break;
                }
            }
        }
    }

    match position {
        Some(pos) => {
            *r_parcel_cursor = ParcelCursor::Point((
                r_selected_parcel.0.unwrap(),
                IVec2::new(pos.x.round() as i32, pos.z.round() as i32),
                0,
            ));
        }
        None => {
            *r_parcel_cursor = ParcelCursor::None;
        }
    }
}

pub fn on_pick_event(
    trigger: Trigger<PickEvent>,
    mut r_parcel_cursor: ResMut<ParcelCursor>,
    mut r_selected_parcel: ResMut<SelectedParcel>,
) {
    let event = trigger.event();
    match event.action {
        PickAction::Leave => {
            *r_parcel_cursor = ParcelCursor::None;
        }
        PickAction::Down(_pos) => {
            // if let PickTarget::Parcel(p) = event.target {
            //     r_selected_parcel.0 = Some(p);
            // }
        }
        PickAction::RightClick => todo!(),
        PickAction::DblClick => todo!(),
        PickAction::DragStart(_pos) => {
            if let PickTarget::Parcel(p) = event.target {
                r_selected_parcel.0 = Some(p);
            }
        }
        PickAction::Drag => {}
        PickAction::DragEnd => {}
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
                    .tint(false),
                ToolIconButton::new("editor/icons/rotate-cw.png")
                    .size(Vec2::new(32., 24.))
                    .tint(false)
                    .corners(RoundedCorners::BottomRight),
            )),
            ListView::new().style(|sb: &mut StyleBuilder| {
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
