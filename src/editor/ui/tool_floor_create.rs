use bevy::prelude::*;
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};
use bevy_quill::View;

use crate::{
    editor::SelectedPrecinct, scenery::precinct::Precinct, view::picking::PickEvent, world::Realm,
};

use super::{mode_scenery::SceneryOverlay, overlays::SelectedPrecinctOverlay};

#[derive(Clone, Component)]
pub struct PrecinctOverlay;

pub fn enter(mut commands: Commands) {
    commands.spawn((SelectedPrecinctOverlay.to_root(), PrecinctOverlay));
    //     // commands.spawn((TerrainCursorOverlay.to_root(), PrecinctOverlay));
    commands.spawn((
        StateScoped(SceneryOverlay::FloorCreate),
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
    mut commands: Commands,
    r_selected_precinct: Res<SelectedPrecinct>,
    // mut r_drag_state: ResMut<TerrainDragState>,
    r_hover_map: Res<HoverMap>,
    // r_tool: Res<State<TerrainTool>>,
    // r_contours_handle: Res<TerrainContoursHandle>,
    // r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
    // q_parcels: Query<&Parcel>,
) {
}

pub fn on_pick_event(
    trigger: Trigger<PickEvent>,
    mut commands: Commands,
    q_realms: Query<&Realm>,
    q_precincts: Query<(Entity, &Precinct)>,
    // r_tool: Res<State<TerrainTool>>,
    mut r_selected_precinct: ResMut<SelectedPrecinct>,
    // mut r_drag_state: ResMut<TerrainDragState>,
    // r_contours_handle: Res<TerrainContoursHandle>,
    // r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
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
            } else {
                println!("Draw");
            }
        }
        crate::view::picking::PickAction::Drag => {}
        crate::view::picking::PickAction::DragEnd => {}
    }
}
