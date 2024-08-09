use bevy::prelude::*;
use bevy_mod_picking::{focus::HoverMap, prelude::PointerId};

use crate::view::picking::PickEvent;

use super::mode_scenery::SceneryOverlay;

#[derive(Clone, Component)]
pub struct PrecinctOverlay;

pub fn enter(mut commands: Commands) {
    //     // commands.spawn((SelectedParcelOverlay.to_root(), PrecinctOverlay));
    //     // commands.spawn((TerrainCursorOverlay.to_root(), PrecinctOverlay));
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
    mut commands: Commands,
    // r_selected_parcel: Res<SelectedParcel>,
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
    // q_parcels: Query<(Entity, &Parcel)>,
    // r_tool: Res<State<TerrainTool>>,
    // mut r_selected_parcel: ResMut<SelectedParcel>,
    // mut r_drag_state: ResMut<TerrainDragState>,
    // r_contours_handle: Res<TerrainContoursHandle>,
    // r_contours_asset: ResMut<Assets<TerrainContoursTableAsset>>,
) {
}
