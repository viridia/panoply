use bevy::prelude::*;
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};
use bevy_quill::View;

use crate::{
    editor::events::{PlaceWalls, RemoveWalls, RotateSelection},
    scenery::{precinct::Precinct, PRECINCT_SIZE_F},
    view::picking::PickEvent,
    world::Realm,
};

use super::{
    mode_scenery::{
        SceneryDragState, SceneryOverlay, SelectedFacing, SelectedPrecinct, SelectedTier, WallSnap,
        WallType,
    },
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
    commands.spawn((
        StateScoped(SceneryOverlay::PlaceWall),
        Observer::new(
            |trigger: Trigger<RotateSelection>, mut facing: ResMut<SelectedFacing>| {
                facing.0 = (facing.0 + trigger.event().0).rem_euclid(4);
            },
        ),
    ));
}

pub fn exit(mut commands: Commands, q_overlays: Query<Entity, With<PrecinctOverlay>>) {
    q_overlays.iter().for_each(|e| commands.entity(e).despawn());
}

pub fn update(
    q_precints: Query<&Precinct>,
    q_realms: Query<&Realm>,
    r_selected_precinct: Res<SelectedPrecinct>,
    r_selected_wall: Res<WallType>,
    mut r_drag_state: ResMut<SceneryDragState>,
    r_hover_map: Res<HoverMap>,
    r_snap: Res<State<WallSnap>>,
) {
    let mut drag_state = r_drag_state.clone();
    drag_state.precinct = r_selected_precinct.0;
    drag_state.cursor_exemplar = None;

    let snap = *r_snap.get();
    if let Some(precinct_id) = r_selected_precinct.0 {
        if let Ok(precinct) = q_precints.get(precinct_id) {
            let precinct_min = precinct.coords.as_vec2() * PRECINCT_SIZE_F;
            if let Some(p) = r_hover_map.get(&PointerId::Mouse) {
                for (_, hit_data) in p.iter() {
                    if let Some(pos) = hit_data.position {
                        let (snap_quanta, snap_offset) = match snap {
                            WallSnap::Normal => (1.0, 0.5),
                            WallSnap::Offset => (1.0, 0.0),
                            WallSnap::Quarter => (0.25, 0.125),
                        };
                        let pickpos = (Vec2::new(
                            (pos.x / snap_quanta + snap_offset).floor() * snap_quanta - snap_offset,
                            (pos.z / snap_quanta + snap_offset).floor() * snap_quanta - snap_offset,
                        ) - precinct_min)
                            .clamp(Vec2::ZERO, Vec2::splat(PRECINCT_SIZE_F));

                        drag_state.cursor_exemplar = r_selected_wall.0;
                        if !drag_state.dragging {
                            drag_state.anchor_height = 0;
                            drag_state.anchor_pos = pickpos;
                            drag_state.cursor_pos = pickpos;
                            drag_state.cursor_layer =
                                q_realms.get(precinct.realm).unwrap().layer_index;
                        } else if drag_state.cursor_pos != pickpos {
                            drag_state.cursor_pos =
                                (pickpos - drag_state.anchor_pos).round() + drag_state.anchor_pos;
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
    r_selected_facing: ResMut<SelectedFacing>,
    r_selected_tier: Res<SelectedTier>,
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
                if let Some(precinct_id) = r_selected_precinct.0 {
                    if let Ok((_, precinct)) = q_precincts.get(precinct_id) {
                        if let Some(exemplar) = r_drag_state.cursor_exemplar {
                            commands.trigger(PlaceWalls {
                                precinct: precinct.asset.clone(),
                                tier: r_selected_tier.0,
                                area: Rect::from_corners(
                                    r_drag_state.cursor_pos,
                                    r_drag_state.anchor_pos,
                                ),
                                facing: (r_selected_facing.0 as f32 * -90.0).rem_euclid(360.0), // Facing is in degrees
                                exemplar,
                                replace: true,
                            });
                        } else {
                            commands.trigger(RemoveWalls {
                                precinct: precinct.asset.clone(),
                                tier: r_selected_tier.0,
                                area: Rect::from_corners(
                                    r_drag_state.cursor_pos,
                                    r_drag_state.anchor_pos,
                                ),
                            });
                        }
                    }
                }
                r_drag_state.dragging = false;
            }
        }
    }
}
