use bevy::prelude::*;
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};
use bevy_quill::View;

use crate::{
    scenery::{precinct::Precinct, PRECINCT_SIZE_F},
    view::picking::PickEvent,
};

use super::{
    mode_scenery::{SceneryDragState, SceneryOverlay, SelectedPrecinct, WallSnap, WallType},
    overlays::{SelectedPrecinctOverlay, WallDrawOverlay},
};

#[derive(Clone, Component)]
pub struct PrecinctOverlay;

pub fn enter(mut commands: Commands) {
    commands.spawn((SelectedPrecinctOverlay.to_root(), PrecinctOverlay));
    commands.spawn((WallDrawOverlay.to_root(), PrecinctOverlay));
    commands.spawn((
        StateScoped(SceneryOverlay::PlaceWall),
        Observer::new(on_pick_event),
    ));
    //     // commands.spawn((
    //     //     StateScoped(EditorMode::Terrain),
    //     //     Observer::new(tool_terrain_edit::on_modify_terrain),
    //     // ));
}

pub fn exit(mut commands: Commands, q_overlays: Query<Entity, With<PrecinctOverlay>>) {
    q_overlays.iter().for_each(|e| commands.entity(e).despawn());
}

pub fn hover(
    // mut commands: Commands,
    q_precints: Query<&Precinct>,
    r_selected_precinct: Res<SelectedPrecinct>,
    mut r_drag_state: ResMut<SceneryDragState>,
    // mut r_drag_state: ResMut<TerrainDragState>,
    r_hover_map: Res<HoverMap>,
    r_snap: Res<State<WallSnap>>,
    // r_contours_handle: Res<TerrainContoursHandle>,
    // r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
    // q_parcels: Query<&Parcel>,
) {
    let mut drag_state = r_drag_state.clone();
    drag_state.precinct = r_selected_precinct.0;
    drag_state.cursor_model = None;
    let snap = *r_snap.get();

    if let Some(precinct_id) = r_selected_precinct.0 {
        if let Ok(precinct) = q_precints.get(precinct_id) {
            let precinct_min = precinct.coords.as_vec2() * PRECINCT_SIZE_F;
            if let Some(p) = r_hover_map.get(&PointerId::Mouse) {
                for (_, hit_data) in p.iter() {
                    if let Some(pos) = hit_data.position {
                        let (snap_quanta, snap_offset) = match snap {
                            WallSnap::Normal => (1.0, 0.0),
                            WallSnap::Offset => (1.0, 0.5),
                            WallSnap::Quarter => (0.25, 0.0),
                        };
                        let pickpos = Vec2::new(
                            (pos.x / snap_quanta + snap_offset).floor() * snap_quanta - snap_offset,
                            (pos.z / snap_quanta + snap_offset).floor() * snap_quanta - snap_offset,
                        ) - precinct_min;

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
    r_selected_floor_type: ResMut<WallType>,
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
                    // commands.trigger(FloorStampEvent {
                    //     precinct,
                    //     tier: 0,
                    //     floor_type: r_selected_floor_type.0,
                    //     shape: vec![r_drag_state.floor_outline.clone()],
                    // });
                }
                r_drag_state.dragging = false;
            }
        }
    }
}
