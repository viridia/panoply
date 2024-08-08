use crate::{
    editor::{
        events::{ChangeContourEvent, ModifyTerrainMapEvent},
        DragShape, EditorMode, SelectedParcel, TerrainDragState, TerrainTool,
    },
    terrain::{
        terrain_contours::{FloraType, TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel, ParcelFloraChanged, ParcelWaterChanged, RebuildParcelGroundMesh, ShapeRef,
        TerrainMap, TerrainMapAsset, PARCEL_SIZE, PARCEL_SIZE_U,
    },
    view::picking::{PickAction, PickEvent, PickTarget},
};
use bevy::{prelude::*, ui};
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, size::Size, RoundedCorners};

use super::{
    controls::ContourChooser,
    overlays::{SelectedParcelOverlay, TerrainCursorOverlay},
};

#[derive(Clone, Component)]
pub struct ParcelOverlay;

pub fn enter(mut commands: Commands) {
    commands.spawn((SelectedParcelOverlay.to_root(), ParcelOverlay));
    commands.spawn((TerrainCursorOverlay.to_root(), ParcelOverlay));
    commands.spawn((
        StateScoped(EditorMode::Terrain),
        Observer::new(on_pick_event),
    ));
    commands.spawn((
        StateScoped(EditorMode::Terrain),
        Observer::new(on_change_terrain),
    ));
}

pub fn exit(mut commands: Commands, q_overlays: Query<Entity, With<ParcelOverlay>>) {
    q_overlays.iter().for_each(|e| commands.entity(e).despawn());
}

pub fn hover(
    mut commands: Commands,
    r_selected_parcel: Res<SelectedParcel>,
    mut r_drag_state: ResMut<TerrainDragState>,
    r_hover_map: Res<HoverMap>,
    r_tool: Res<State<TerrainTool>>,
    r_contours_handle: Res<TerrainContoursHandle>,
    r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
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
                                        drag_state.drag_shape = DragShape::Point;
                                        drag_state.cursor_pos = pickpos;
                                    } else if drag_state.cursor_pos != pickpos {
                                        drag_state.cursor_pos = pickpos;
                                        modify_terrain(
                                            *tool,
                                            &drag_state,
                                            parcel,
                                            r_contours_handle,
                                            r_contours_asset,
                                        );
                                        commands.trigger(ChangeContourEvent(
                                            parcel.center_shape().shape as usize,
                                        ));
                                    }
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
                                                lock.get(shape_ref.shape as usize)
                                                    .unscaled_height_at(
                                                        pickpos.x.clamp(0, PARCEL_SIZE_U as i32)
                                                            as usize,
                                                        pickpos.y.clamp(0, PARCEL_SIZE_U as i32)
                                                            as usize,
                                                        shape_ref.rotation,
                                                    )
                                            })
                                        else {
                                            continue;
                                        };

                                        drag_state.anchor_height = cursor_height;
                                        drag_state.anchor_pos = pickpos;
                                        drag_state.drag_shape = DragShape::FlatRect;
                                    }
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
                                    if drag_state.dragging {
                                        if drag_state.cursor_pos != pickpos {
                                            drag_state.cursor_pos = pickpos;
                                            drag_state.anchor_pos = pickpos;
                                            modify_terrain(
                                                *tool,
                                                &drag_state,
                                                parcel,
                                                r_contours_handle,
                                                r_contours_asset,
                                            );
                                            commands.entity(parcel_id).insert(ParcelFloraChanged);
                                            commands.trigger(ChangeContourEvent(
                                                parcel.center_shape().shape as usize,
                                            ));
                                        }
                                    } else {
                                        drag_state.cursor_pos = pickpos;
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
    mut commands: Commands,
    q_parcels: Query<(Entity, &Parcel)>,
    r_tool: Res<State<TerrainTool>>,
    mut r_selected_parcel: ResMut<SelectedParcel>,
    mut r_drag_state: ResMut<TerrainDragState>,
    r_contours_handle: Res<TerrainContoursHandle>,
    r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
) {
    let event = trigger.event();
    let tool = r_tool.get();
    match event.action {
        PickAction::Leave => {
            // *r_parcel_cursor = ParcelCursor::None;
            r_drag_state.drag_shape = DragShape::None;
        }
        PickAction::Down(_pos) => {}
        PickAction::RightClick => todo!(),
        PickAction::DblClick => todo!(),
        PickAction::DragStart(_pos) => {
            if let PickTarget::Parcel(p) = event.target {
                if r_selected_parcel.0 != Some(p) {
                    r_selected_parcel.0 = Some(p);
                    r_drag_state.parcel = Some(p);
                } else if let Some(parcel_id) = r_selected_parcel.0 {
                    r_drag_state.dragging = true;
                    match tool {
                        TerrainTool::RaiseDraw
                        | TerrainTool::LowerDraw
                        | TerrainTool::FlattenDraw
                        | TerrainTool::FlattenRect
                        | TerrainTool::DrawTrees
                        | TerrainTool::DrawShrubs
                        | TerrainTool::DrawHerbs
                        | TerrainTool::EraseFlora => {
                            let parcel = q_parcels.get(parcel_id).unwrap().1;
                            modify_terrain(
                                *tool,
                                &r_drag_state,
                                parcel,
                                r_contours_handle,
                                r_contours_asset,
                            );
                            commands.entity(parcel_id).insert((
                                RebuildParcelGroundMesh,
                                ParcelWaterChanged,
                                ParcelFloraChanged,
                            ));
                            commands
                                .trigger(ChangeContourEvent(parcel.center_shape().shape as usize));
                        }

                        _ => {}
                    }
                }
                r_drag_state.anchor_pos = r_drag_state.cursor_pos;
            }
        }
        PickAction::Drag => {}
        PickAction::DragEnd => {
            if let Some(parcel_id) = r_selected_parcel.0 {
                if let Ok((_, parcel)) = q_parcels.get(parcel_id) {
                    match tool {
                        TerrainTool::RaiseRect
                        | TerrainTool::LowerRect
                        | TerrainTool::FlattenRect => {
                            modify_terrain(
                                *tool,
                                &r_drag_state,
                                parcel,
                                r_contours_handle,
                                r_contours_asset,
                            );
                            commands
                                .trigger(ChangeContourEvent(parcel.center_shape().shape as usize));
                        }

                        _ => {}
                    }
                    let shape_ref = parcel.center_shape();
                    for (parcel_id, parcel) in q_parcels.iter() {
                        if parcel.has_shape(shape_ref.shape) {
                            commands.entity(parcel_id).insert((
                                RebuildParcelGroundMesh,
                                ParcelWaterChanged,
                                ParcelFloraChanged,
                            ));
                        }
                    }
                }
            }
            r_drag_state.dragging = false;
        }
    }
}

pub fn on_change_terrain(
    trigger: Trigger<ModifyTerrainMapEvent>,
    q_parcels: Query<&Parcel>,
    q_terrain_map: Query<&TerrainMap>,
    mut r_terrain_map_assets: ResMut<Assets<TerrainMapAsset>>,
    r_selected_parcel: Res<SelectedParcel>,
) {
    let Some(parcel_id) = r_selected_parcel.0 else {
        return;
    };
    let Ok(parcel) = q_parcels.get(parcel_id) else {
        return;
    };
    let Ok(terrain_map) = q_terrain_map.get(parcel.realm) else {
        warn!("No terrain map for realm: {:?}", parcel.realm);
        return;
    };
    let terrain_map_asset = r_terrain_map_assets.get_mut(&terrain_map.handle).unwrap();
    terrain_map_asset.set_shape_at(parcel.coords, trigger.event().shape);
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
                pt = pt.clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE - 1));
            }
            if pt.x >= 0 && pt.x < PARCEL_SIZE && pt.y >= 0 && pt.y < PARCEL_SIZE {
                Some(pt)
            } else {
                None
            }
        }
    }
}

fn modify_terrain(
    tool: TerrainTool,
    drag_state: &TerrainDragState,
    parcel: &Parcel,
    r_contours_handle: Res<TerrainContoursHandle>,
    mut r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
) {
    match tool {
        TerrainTool::RaiseDraw | TerrainTool::LowerDraw | TerrainTool::FlattenDraw => {
            let new_height = match tool {
                TerrainTool::RaiseDraw => drag_state.anchor_height + 1,
                TerrainTool::LowerDraw => drag_state.anchor_height - 1,
                TerrainTool::FlattenDraw => drag_state.anchor_height,
                _ => unreachable!(),
            }
            .clamp(i8::MIN as i32, i8::MAX as i32);
            let shape_ref = parcel.center_shape();
            if let Some(contours) = r_contours_asset.get_mut(&r_contours_handle.0) {
                let mut lock = contours.0.write().unwrap();
                let pos = drag_state
                    .cursor_pos
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
        TerrainTool::RaiseRect | TerrainTool::LowerRect | TerrainTool::FlattenRect => {
            let new_height = match tool {
                TerrainTool::RaiseRect => drag_state.anchor_height + 1,
                TerrainTool::LowerRect => drag_state.anchor_height - 1,
                TerrainTool::FlattenRect => drag_state.anchor_height,
                _ => unreachable!(),
            }
            .clamp(i8::MIN as i32, i8::MAX as i32);
            let shape_ref = parcel.center_shape();
            if let Some(contours) = r_contours_asset.get_mut(&r_contours_handle.0) {
                let mut lock = contours.0.write().unwrap();
                let rect =
                    IRect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos).intersect(
                        IRect::from_corners(IVec2::ZERO, IVec2::splat(PARCEL_SIZE - 1)),
                    );
                let contour = lock.get_mut(shape_ref.shape as usize);
                for x in rect.min.x..=rect.max.x {
                    for y in rect.min.y..=rect.max.y {
                        contour.set_height_at(
                            x as usize,
                            y as usize,
                            shape_ref.rotation,
                            new_height as i8,
                        );
                    }
                }
            }
        }
        TerrainTool::DrawTrees
        | TerrainTool::DrawShrubs
        | TerrainTool::DrawHerbs
        | TerrainTool::EraseFlora => {
            let shape_ref = parcel.center_shape();
            if let Some(contours) = r_contours_asset.get_mut(&r_contours_handle.0) {
                let mut lock = contours.0.write().unwrap();
                let pos = drag_state
                    .cursor_pos
                    .clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE - 1));
                let contour = lock.get_mut(shape_ref.shape as usize);
                contour.set_flora_at(
                    pos.x as usize,
                    pos.y as usize,
                    shape_ref.rotation,
                    match tool {
                        TerrainTool::DrawTrees => FloraType::RandomTree,
                        TerrainTool::DrawShrubs => FloraType::RandomShrub,
                        TerrainTool::DrawHerbs => FloraType::RandomHerb,
                        TerrainTool::EraseFlora => FloraType::None,
                        _ => unreachable!(),
                    },
                );
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
