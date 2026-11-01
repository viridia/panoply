//! View readiness, for test plans: whether everything around the viewpoint has loaded and been
//! built, so a plan can wait for it after moving the view instead of waiting a guessed number
//! of frames.

use bevy::prelude::*;
use panoply_core::{Realm, Viewpoint};
use panoply_render::ModelsPending;
use panoply_scenery::SceneryPending;
use panoply_terrain::TerrainPending;

/// Whether the view, as of this frame, has finished loading. Updated in `Last`, read over BRP.
/// Covers every loaded parcel and precinct, not only those on screen, so it is a little stricter
/// than the view needs. Loading follows the viewpoint a frame late, so a viewpoint that moved
/// this frame is never ready.
#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource)]
pub struct ViewReady {
    /// The viewpoint this applies to: realm name, position, camera distance.
    pub realm: String,
    pub position: Vec3,
    pub camera_distance: f32,
    pub ready: bool,
    /// What is still pending when not ready, such as `"parcels"`; empty when ready.
    pub waiting_on: String,
}

pub(super) fn update_view_ready(
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    terrain: TerrainPending,
    scenery: SceneryPending,
    models: ModelsPending,
    mut view_ready: ResMut<ViewReady>,
) {
    let realm = viewpoint
        .realm
        .and_then(|realm| realms.get(realm).ok())
        .map_or("", |realm| realm.name.as_str());
    let moved = view_ready.realm != realm
        || view_ready.position != viewpoint.position
        || view_ready.camera_distance != viewpoint.camera_distance();
    let waiting_on = if realm.is_empty() {
        Some("realm")
    } else if moved {
        Some("viewpoint")
    } else {
        terrain
            .waiting_on()
            .or_else(|| scenery.waiting_on())
            .or_else(|| models.waiting_on())
    };
    if moved {
        view_ready.realm = realm.to_string();
        view_ready.position = viewpoint.position;
        view_ready.camera_distance = viewpoint.camera_distance();
    }
    view_ready.ready = waiting_on.is_none();
    view_ready.waiting_on = waiting_on.unwrap_or_default().to_string();
}
