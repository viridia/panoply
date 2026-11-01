//! Readiness survey, for test plans: the BRP method `panoply.readiness` reports, for every loaded
//! parcel and precinct, what is still loading or building. A plan prints it as a grid to see
//! where loading stalls, or how far around the view it reaches. Computed only when called.

use bevy::{
    prelude::*,
    remote::{BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use panoply_core::{Realm, Viewpoint};
use panoply_render::ModelsPending;
use panoply_scenery::SceneryPending;
use panoply_terrain::TerrainPending;
use serde_json::{json, Value};

use super::ViewReady;

/// The method's name over BRP.
pub const METHOD: &str = "panoply.readiness";

/// Registers the method, when the game runs with BRP (`src/remote_driver.rs`).
pub(super) fn register(world: &mut World) {
    if !world.contains_resource::<RemoteMethods>() {
        return;
    }
    let system = world.register_system(survey);
    world
        .resource_mut::<RemoteMethods>()
        .insert(METHOD, RemoteMethodSystemId::Instant(system));
}

fn survey(
    In(_params): In<Option<Value>>,
    viewpoint: Res<Viewpoint>,
    view_ready: Res<ViewReady>,
    realms: Query<&Realm>,
    terrain: TerrainPending,
    scenery: SceneryPending,
    models: ModelsPending,
) -> BrpResult {
    let realm_name = |realm: Entity| realms.get(realm).map_or("", |realm| realm.name.as_str());

    let mut parcels = terrain.survey();
    parcels.sort_by_key(|p| (p.realm, p.coords.y, p.coords.x));
    let parcels: Vec<Value> = parcels
        .iter()
        .map(|p| {
            json!({
                "realm": realm_name(p.realm),
                "x": p.coords.x,
                "z": p.coords.y,
                "shown": p.shown,
                "pending": p.pending,
            })
        })
        .collect();

    let mut precincts = scenery.survey();
    precincts.sort_by_key(|p| (p.realm, p.coords.y, p.coords.x));
    let precincts: Vec<Value> = precincts
        .iter()
        .map(|p| {
            json!({
                "realm": realm_name(p.realm),
                "x": p.coords.x,
                "z": p.coords.y,
                "asset": p.asset,
                "pending": p.pending,
            })
        })
        .collect();

    Ok(json!({
        "viewpoint": {
            "realm": viewpoint.realm.map_or("", realm_name),
            "position": viewpoint.position.to_array(),
            "camera_distance": viewpoint.camera_distance(),
        },
        "ready": view_ready.ready,
        "waiting_on": view_ready.waiting_on,
        "terrain_assets_loading": terrain.assets_loading(),
        "models": models.waiting_on(),
        "parcels": parcels,
        "precincts": precincts,
    }))
}
