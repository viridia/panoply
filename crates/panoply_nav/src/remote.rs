//! The BRP method `panoply.nav_input`, for test plans: captures a loaded precinct's navigation
//! input and writes it to a file, as the generators' tests read it, and reports what it holds.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    remote::{error_codes, BrpError, BrpResult, RemoteMethodHandler, RemoteMethods},
};
use panoply_core::Realm;
use panoply_scenery::ColliderType;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{NavInputCapture, SurfaceKind};

/// The method's name over BRP.
pub const METHOD: &str = "panoply.nav_input";

/// The method's parameters: the precinct by realm name and coordinates, and the file to write.
#[derive(Deserialize)]
struct Params {
    realm: String,
    x: i32,
    z: i32,
    path: PathBuf,
}

/// Registers the method, when the game runs with BRP.
pub(crate) fn register(world: &mut World) {
    if !world.contains_resource::<RemoteMethods>() {
        return;
    }
    let method =
        RemoteMethodHandler::Instant(Box::new(IntoSystem::into_system(nav_input))).register(world);
    world.resource_mut::<RemoteMethods>().insert(METHOD, method);
}

fn nav_input(
    In(params): In<Option<Value>>,
    realms: Query<(Entity, &Realm)>,
    capture: NavInputCapture,
) -> BrpResult {
    let error = |message: String| BrpError {
        code: error_codes::INTERNAL_ERROR,
        message,
        data: None,
    };
    let params: Params =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| BrpError {
            code: error_codes::INVALID_PARAMS,
            message: e.to_string(),
            data: None,
        })?;
    let realm = realms
        .iter()
        .find(|(_, realm)| realm.name == params.realm)
        .map(|(entity, _)| entity)
        .ok_or_else(|| error(format!("no realm {}", params.realm)))?;
    let input = capture
        .capture(realm, IVec2::new(params.x, params.z))
        .map_err(|e| error(e.to_string()))?;
    input.write(&params.path).map_err(error)?;

    let surfaces = |kind| input.surfaces.iter().filter(|s| s.kind == kind).count();
    let colliders = |kind| {
        input
            .colliders
            .iter()
            .filter(|c| c.collider.kind == kind)
            .count()
    };
    Ok(json!({
        "ground": surfaces(SurfaceKind::Ground),
        "floors": surfaces(SurfaceKind::Floor),
        "colliders": input.colliders.len(),
        "doors": colliders(ColliderType::Door),
        "hints": colliders(ColliderType::Hint),
        "path_cells": input.paths.iter().filter(|&&p| p).count(),
    }))
}
