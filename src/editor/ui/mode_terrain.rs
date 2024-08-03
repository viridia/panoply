use crate::{
    editor::{DragShape, EditorMode, SelectedParcel, TerrainDragState, TerrainTool},
    terrain::{
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel, ParcelFloraChanged, RebuildParcelGroundMesh, PARCEL_SIZE, PARCEL_SIZE_U,
    },
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
    mut commands: Commands,
    r_selected_parcel: Res<SelectedParcel>,
    mut r_drag_state: ResMut<TerrainDragState>,
    r_hover_map: Res<HoverMap>,
    r_tool: Res<State<TerrainTool>>,
    r_contours_handle: Res<TerrainContoursHandle>,
    mut r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
    q_parcels: Query<&Parcel>,
) {
    let mut drag_state = r_drag_state.clone();
    let tool = r_tool.get();
    if !drag_state.dragging {
        drag_state.drag_shape = DragShape::None;
    }
    if let Some(parcel_id) = r_selected_parcel.0 {
        if let Ok(parcel) = q_parcels.get(parcel_id) {
            let parcel_min = parcel.coords * PARCEL_SIZE;
            if let Some(p) = r_hover_map.get(&PointerId::Mouse) {
                for (_, hit_data) in p.iter() {
                    if let Some(pos) = hit_data.position {
                        let rpos = Vec2::new(pos.x, pos.z) - parcel_min.as_vec2();
                        match tool {
                            TerrainTool::RaiseDraw
                            | TerrainTool::LowerDraw
                            | TerrainTool::FlattenDraw => {
                                if let Some(pickpos) =
                                    terrain_pick_pos(DragShape::Point, rpos, true)
                                {
                                    if !drag_state.dragging {
                                        let shape_ref = parcel.center_shape();
                                        let Some(cursor_height) = r_contours_asset
                                            .get(&r_contours_handle.0)
                                            .map(|contours| {
                                                let lock = contours.0.read().unwrap();
                                                let pos = pickpos - parcel_min;
                                                lock.get(shape_ref.shape as usize)
                                                    .unscaled_height_at(
                                                        pos.x.clamp(0, PARCEL_SIZE_U as i32)
                                                            as usize,
                                                        pos.y.clamp(0, PARCEL_SIZE_U as i32)
                                                            as usize,
                                                        shape_ref.rotation,
                                                    )
                                            })
                                        else {
                                            continue;
                                        };
                                        drag_state.anchor_height = cursor_height;
                                        drag_state.anchor_pos = pickpos;
                                    } else {
                                        let new_height = match tool {
                                            TerrainTool::RaiseDraw => drag_state.anchor_height + 1,
                                            TerrainTool::LowerDraw => drag_state.anchor_height - 1,
                                            TerrainTool::FlattenDraw => drag_state.anchor_height,
                                            _ => unreachable!(),
                                        }
                                        .clamp(i8::MIN as i32, i8::MAX as i32);
                                        let shape_ref = parcel.center_shape();
                                        if let Some(contours) =
                                            r_contours_asset.get_mut(&r_contours_handle.0)
                                        {
                                            let mut lock = contours.0.write().unwrap();
                                            let pos = (pickpos - parcel_min)
                                                .clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE));
                                            let contour = lock.get_mut(shape_ref.shape as usize);
                                            contour.set_height_at(
                                                pos.x as usize,
                                                pos.y as usize,
                                                shape_ref.rotation,
                                                new_height as i8,
                                            );
                                        }
                                        commands.entity(parcel_id).insert((
                                            RebuildParcelGroundMesh,
                                            // ParcelWaterChanged,
                                            ParcelFloraChanged,
                                        ));
                                    }
                                    drag_state.drag_shape = DragShape::Point;
                                    drag_state.cursor_pos = pickpos;
                                    break;
                                }
                            }
                            TerrainTool::RaiseRect
                            | TerrainTool::LowerRect
                            | TerrainTool::FlattenRect => {
                                if let Some(pickpos) =
                                    terrain_pick_pos(DragShape::FlatRect, rpos, true)
                                {
                                    if !drag_state.dragging {
                                        let shape_ref = parcel.center_shape();
                                        let Some(cursor_height) = r_contours_asset
                                            .get(&r_contours_handle.0)
                                            .map(|contours| {
                                                let lock = contours.0.read().unwrap();
                                                let pos = pickpos - parcel_min;
                                                lock.get(shape_ref.shape as usize)
                                                    .unscaled_height_at(
                                                        pos.x.clamp(0, PARCEL_SIZE_U as i32)
                                                            as usize,
                                                        pos.y.clamp(0, PARCEL_SIZE_U as i32)
                                                            as usize,
                                                        shape_ref.rotation,
                                                    )
                                            })
                                        else {
                                            continue;
                                        };

                                        drag_state.anchor_height = cursor_height;
                                        drag_state.anchor_pos = pickpos;
                                    } else {
                                        let new_height = match tool {
                                            TerrainTool::RaiseRect => drag_state.anchor_height + 1,
                                            TerrainTool::LowerRect => drag_state.anchor_height - 1,
                                            TerrainTool::FlattenRect => drag_state.anchor_height,
                                            _ => unreachable!(),
                                        }
                                        .clamp(i8::MIN as i32, i8::MAX as i32);
                                        let shape_ref = parcel.center_shape();
                                        if let Some(contours) =
                                            r_contours_asset.get_mut(&r_contours_handle.0)
                                        {
                                            let mut lock = contours.0.write().unwrap();
                                            let pos = (pickpos - parcel_min)
                                                .clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE));
                                            let contour = lock.get_mut(shape_ref.shape as usize);
                                            contour.set_height_at(
                                                pos.x as usize,
                                                pos.y as usize,
                                                shape_ref.rotation,
                                                new_height as i8,
                                            );
                                        }
                                    }
                                    drag_state.drag_shape = DragShape::FlatRect;
                                    drag_state.cursor_pos = pickpos;
                                    break;
                                }
                            }
                            TerrainTool::DrawTrees
                            | TerrainTool::DrawShrubs
                            | TerrainTool::DrawHerbs
                            | TerrainTool::EraseFlora => {
                                if let Some(pickpos) =
                                    terrain_pick_pos(DragShape::DecalRect, rpos, true)
                                {
                                    drag_state.drag_shape = DragShape::DecalRect;
                                    drag_state.cursor_pos = pickpos;
                                    if !drag_state.dragging {
                                        drag_state.anchor_pos = pickpos;
                                    }
                                    break;
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    if *r_drag_state != drag_state {
        *r_drag_state = drag_state;
    }
}

pub fn on_pick_event(
    trigger: Trigger<PickEvent>,
    mut r_selected_parcel: ResMut<SelectedParcel>,
    mut r_drag_state: ResMut<TerrainDragState>,
) {
    let event = trigger.event();
    match event.action {
        PickAction::Leave => {
            // *r_parcel_cursor = ParcelCursor::None;
            r_drag_state.drag_shape = DragShape::None;
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
                if r_selected_parcel.0 != Some(p) {
                    r_selected_parcel.0 = Some(p);
                    r_drag_state.parcel = Some(p);
                } else {
                    r_drag_state.dragging = true;
                }
                r_drag_state.anchor_pos = r_drag_state.cursor_pos;
            }
        }
        PickAction::Drag => {}
        PickAction::DragEnd => {
            r_drag_state.dragging = false;
        }
    }
}

fn terrain_pick_pos(drag_shape: DragShape, pos: Vec2, clamp: bool) -> Option<IVec2> {
    match drag_shape {
        DragShape::None => None,
        DragShape::Point | DragShape::FlatRect => {
            let mut pt = IVec2::new(pos.x.round() as i32, pos.y.round() as i32);
            if clamp {
                pt = pt.clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE));
            }
            if pt.x >= 0 && pt.x <= PARCEL_SIZE && pt.y >= 0 && pt.y <= PARCEL_SIZE {
                Some(pt)
            } else {
                None
            }
        }
        DragShape::DecalRect => {
            let mut pt = IVec2::new(pos.x.floor() as i32, pos.y.floor() as i32);
            if clamp {
                pt = pt.clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE));
            }
            if pt.x >= 0 && pt.x <= PARCEL_SIZE && pt.y >= 0 && pt.y <= PARCEL_SIZE {
                Some(pt)
            } else {
                None
            }
        }
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
