use bevy::prelude::*;
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};
use bevy_quill::View;
use i_overlay::{
    core::{fill_rule::FillRule, overlay::ShapeType, overlay_rule::OverlayRule},
    i_float::point::IntPoint,
};

use crate::{
    editor::{events::FloorStampEvent, lib::pick_plane::PlanePick},
    scenery::{
        floor_region::FloorRegionSer, precinct::Precinct, precinct_asset::PrecinctAsset,
        PRECINCT_SIZE_F,
    },
    view::picking::PickEvent,
};

use super::{
    mode_scenery::{FloorTool, FloorType, SceneryDragState, SceneryOverlay, SelectedPrecinct},
    overlays::{FloorStampOverlay, SelectedPrecinctOverlay},
};

#[derive(Clone, Component)]
pub struct PrecinctOverlay;

pub fn enter(mut commands: Commands, q_camera: Query<Entity, With<crate::view::PrimaryCamera>>) {
    commands.spawn((SelectedPrecinctOverlay.to_root(), PrecinctOverlay));
    commands.spawn((FloorStampOverlay.to_root(), PrecinctOverlay));
    commands.spawn((
        Name::new("FloorPickObserver"),
        StateScoped(SceneryOverlay::FloorCreate),
        Observer::new(on_pick_event),
    ));

    for camera in q_camera.iter() {
        commands.entity(camera).insert(PlanePick);
    }
    commands.spawn((
        Name::new("FloorStampObserver"),
        StateScoped(SceneryOverlay::FloorCreate),
        Observer::new(on_stamp_floor),
    ));
}

pub fn exit(
    mut commands: Commands,
    q_overlays: Query<Entity, With<PrecinctOverlay>>,
    q_camera: Query<Entity, With<crate::view::PrimaryCamera>>,
) {
    q_overlays.iter().for_each(|e| commands.entity(e).despawn());
    for camera in q_camera.iter() {
        commands.entity(camera).remove::<PlanePick>();
    }
}

pub fn hover(
    r_selected_precinct: Res<SelectedPrecinct>,
    mut r_drag_state: ResMut<SceneryDragState>,
    r_hover_map: Res<HoverMap>,
    r_tool: Res<State<FloorTool>>,
    q_precints: Query<&Precinct>,
) {
    let mut drag_state = r_drag_state.clone();
    drag_state.precinct = r_selected_precinct.0;
    drag_state.cursor_model = None;
    let tool = *r_tool.get();
    if let Some(precinct_id) = r_selected_precinct.0 {
        if let Ok(precinct) = q_precints.get(precinct_id) {
            let precinct_min = precinct.coords.as_vec2() * PRECINCT_SIZE_F;
            if let Some(p) = r_hover_map.get(&PointerId::Mouse) {
                for (_, hit_data) in p.iter() {
                    if let Some(pos) = hit_data.position {
                        let pickpos =
                            Vec2::new((pos.x * 2.).round() * 0.5, (pos.z * 2.).round() * 0.5)
                                - precinct_min;
                        // let rpos = Vec2::new(pos.x, pos.z) - precinct_min.as_vec2();
                        if !drag_state.dragging {
                            drag_state.anchor_height = 0;
                            drag_state.anchor_pos = pickpos;
                            // drag_state.drag_shape = DragShape::Point;
                            drag_state.cursor_pos = pickpos;
                        } else if drag_state.cursor_pos != pickpos {
                            drag_state.cursor_pos = pickpos;
                            // modify_terrain(
                            //     *tool,
                            //     &drag_state,
                            //     precinct,
                            //     r_contours_handle,
                            //     r_contours_asset,
                            // );
                            // commands.trigger(ChangeContourEvent(
                            //     precinct.center_shape().shape as usize,
                            // ));
                        }
                        compute_floor_outline(&mut drag_state, tool);
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
    q_precincts: Query<(Entity, &Precinct)>,
    mut r_selected_precinct: ResMut<SelectedPrecinct>,
    r_selected_floor_type: ResMut<FloorType>,
    mut r_drag_state: ResMut<SceneryDragState>,
) {
    let event = trigger.event();

    match event.action {
        crate::view::picking::PickAction::Leave => {}
        crate::view::picking::PickAction::Down(_) => {}
        crate::view::picking::PickAction::RightClick => {}
        crate::view::picking::PickAction::DblClick => {}
        crate::view::picking::PickAction::DragStart { realm, pos } => {
            let precinct = q_precincts
                .iter()
                .find(|(_, p)| p.realm == realm && p.contains_pt(pos));
            let precinct_id = precinct.map(|(e, _)| e);
            if r_selected_precinct.0 != precinct_id {
                r_selected_precinct.0 = precinct_id;
            } else if precinct_id.is_some() {
                r_drag_state.dragging = true;
            }
        }
        crate::view::picking::PickAction::Drag => {}
        crate::view::picking::PickAction::DragEnd => {
            if r_drag_state.dragging {
                if let Some(precinct) = r_selected_precinct.0 {
                    commands.trigger(FloorStampEvent {
                        precinct,
                        tier: 0,
                        floor_type: r_selected_floor_type.0,
                        shape: vec![r_drag_state.floor_outline.clone()],
                    });
                }
                r_drag_state.dragging = false;
            }
        }
    }
}

fn compute_floor_outline(drag_state: &mut SceneryDragState, tool: FloorTool) {
    let vertices = match tool {
        FloorTool::Move | FloorTool::Draw => unreachable!(),
        FloorTool::RectM => {
            let rect =
                Rect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos).inflate(0.5);
            rect_to_vertices(rect)
        }
        FloorTool::RectL => {
            let rect =
                Rect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos).inflate(0.75);
            rect_to_vertices(rect)
        }
        FloorTool::RectXL => {
            let rect =
                Rect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos).inflate(1.0);
            rect_to_vertices(rect)
        }
        FloorTool::Beveled => {
            let r = Rect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos);
            vec![
                Vec2::new(r.min.x, r.min.y - 1.0),
                Vec2::new(r.min.x - 1.0, r.min.y),
                Vec2::new(r.min.x - 1.0, r.max.y),
                Vec2::new(r.min.x, r.max.y + 1.0),
                Vec2::new(r.max.x, r.max.y + 1.0),
                Vec2::new(r.max.x + 1.0, r.max.y),
                Vec2::new(r.max.x + 1.0, r.min.y),
                Vec2::new(r.max.x, r.min.y - 1.0),
            ]
        }
    };
    let mut ishape = i_overlay::core::overlay::Overlay::new(32);
    ishape.add_path(
        &vertices.iter().map(vec2_to_intpoint).collect::<Vec<_>>(),
        ShapeType::Subject,
    );
    let bounds: Rect = Rect::from_corners(Vec2::ZERO, Vec2::splat(PRECINCT_SIZE_F));
    ishape.add_path(
        &rect_to_vertices(bounds)
            .iter()
            .map(vec2_to_intpoint)
            .collect::<Vec<_>>(),
        ShapeType::Clip,
    );
    let graph = ishape.into_graph(FillRule::NonZero);
    let v = graph.extract_shapes(OverlayRule::Intersect);
    for shape in v.iter() {
        for path in shape.iter() {
            drag_state.floor_outline = path.iter().map(intpoint_to_vec2).collect::<Vec<_>>();
        }
    }
}

fn rect_to_vertices(r: Rect) -> Vec<Vec2> {
    vec![
        Vec2::new(r.min.x, r.min.y),
        Vec2::new(r.min.x, r.max.y),
        Vec2::new(r.max.x, r.max.y),
        Vec2::new(r.max.x, r.min.y),
    ]
}

fn vec2_to_intpoint(v: &Vec2) -> IntPoint {
    IntPoint::new((v.x * 256.0) as i32, (v.y * 256.0) as i32)
}

fn intpoint_to_vec2(p: &IntPoint) -> Vec2 {
    Vec2::new((p.x as f32) / 256.0, (p.y as f32) / 256.0)
}

pub fn on_stamp_floor(
    trigger: Trigger<FloorStampEvent>,
    mut q_precincts: Query<&mut Precinct>,
    mut r_precinct_assets: ResMut<Assets<PrecinctAsset>>,
    r_server: ResMut<AssetServer>,
) {
    let event = trigger.event();
    let Ok(mut precinct) = q_precincts.get_mut(event.precinct) else {
        return;
    };
    let precinct_asset = match r_precinct_assets.get_mut(precinct.asset.id()) {
        Some(precinct_asset) => precinct_asset,
        None => {
            let precinct_asset = PrecinctAsset::default();
            precinct.asset = r_precinct_assets.add(precinct_asset);
            r_precinct_assets.get_mut(precinct.asset.id()).unwrap()
        }
    };
    let surface_index = match event.floor_type {
        Some(ft) => {
            let floor_type_path = r_server.get_path(ft).unwrap().to_string();
            match precinct_asset.floor_type_index(&floor_type_path) {
                Some(index) => index,
                None => precinct_asset.add_floor_type(floor_type_path.clone()),
            }
        }
        None => usize::MAX,
    };
    let surface_ct = precinct_asset.floor_types.len();
    let pfloors_old = match precinct_asset.find_tier(event.tier) {
        Some(tier) => tier.pfloors.clone(),
        None => Vec::new(),
    };
    let mut pfloors_new: Vec<FloorRegionSer> = Vec::with_capacity(pfloors_old.capacity());
    for surface in 0..surface_ct {
        let mut ishape = i_overlay::core::overlay::Overlay::new(128);
        for region in pfloors_old.iter() {
            if region.surface_index == surface {
                let mut shape: Vec<Vec<IntPoint>> = Vec::with_capacity(1 + region.holes.len());
                shape.push(
                    region
                        .poly
                        .iter()
                        .rev()
                        .map(vec2_to_intpoint)
                        .collect::<Vec<_>>(),
                );
                shape.extend(
                    region
                        .holes
                        .iter()
                        .map(|hole| hole.iter().rev().map(vec2_to_intpoint).collect::<Vec<_>>()),
                );
                ishape.add_shape(&shape, ShapeType::Subject);
            }

            // Map event shape to overlay shape by converting the vertices to IntPoint.
            ishape.add_shape(
                &event
                    .shape
                    .iter()
                    .map(|path| path.iter().map(vec2_to_intpoint).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
                ShapeType::Clip,
            );
        }

        let graph = ishape.into_graph(FillRule::NonZero);
        let v = if surface == surface_index {
            graph.extract_shapes(OverlayRule::Union)
        } else {
            graph.extract_shapes(OverlayRule::Difference)
        };

        for shape in v.iter() {
            let poly = shape
                .first()
                .unwrap()
                .iter()
                .rev()
                .map(intpoint_to_vec2)
                .collect::<Vec<_>>();
            let holes = shape
                .iter()
                .skip(1)
                .map(|path| path.iter().rev().map(intpoint_to_vec2).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            pfloors_new.push(FloorRegionSer {
                surface_index: surface,
                poly,
                holes,
            });
        }
    }

    // println!("pfloors_new: {:?}", pfloors_new);
    if pfloors_old != pfloors_new {
        let tier = match precinct_asset.find_tier_mut(event.tier) {
            Some(tier) => tier,
            None => {
                if event.floor_type.is_none() {
                    return;
                }
                precinct_asset.add_tier(event.tier)
            }
        };
        tier.pfloors.clone_from(&pfloors_new);
    } else {
        println!("No change to floors");
    }
}
