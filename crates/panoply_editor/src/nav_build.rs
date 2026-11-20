//! Builds the loaded precincts' navigation meshes for the navigation view (`nav_view`): each
//! precinct without a mesh when Navigation turns on, and each that loads while it is on, once the
//! precinct and its parcels are ready. Built once: an edit does not rebuild a mesh, and a mesh
//! removed stays removed.

use bevy::{
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};
use panoply_core::Viewpoint;
use panoply_nav::{clipping, NavInputCapture, NavMesh, PrecinctNavMesh};
use panoply_scenery::{precinct::Precinct, SceneryPending, PRECINCT_SIZE_F};
use panoply_terrain::TerrainPending;

use crate::debug_views::DebugViews;

/// A precinct waiting to be ready for its mesh to be built.
#[derive(Component)]
struct WantsNavMesh;

/// A precinct's mesh being built.
#[derive(Component)]
struct NavMeshBuild(Task<NavMesh>);

pub struct NavBuildPlugin;

impl Plugin for NavBuildPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                want_meshes.run_if(resource_changed::<DebugViews>),
                want_new_precincts,
                start_builds,
                finish_builds,
            )
                .chain(),
        );
    }
}

/// When Navigation turns on, every loaded precinct without a mesh wants one.
fn want_meshes(
    views: Res<DebugViews>,
    mut was_on: Local<bool>,
    precincts: Query<Entity, (With<Precinct>, Without<PrecinctNavMesh>)>,
    mut commands: Commands,
) {
    let turned_on = views.navigation && !*was_on;
    *was_on = views.navigation;
    if turned_on {
        for entity in &precincts {
            commands.entity(entity).insert(WantsNavMesh);
        }
    }
}

/// While Navigation is on, a precinct that loads wants a mesh.
fn want_new_precincts(
    views: Res<DebugViews>,
    precincts: Query<Entity, Added<Precinct>>,
    mut commands: Commands,
) {
    if views.navigation {
        for entity in &precincts {
            commands.entity(entity).insert(WantsNavMesh);
        }
    }
}

/// Captures the input of each wanting precinct in the viewpoint's realm whose colliders are ready,
/// and builds its mesh off the main thread. Only the colliders navigation reads are waited on, as
/// physics on that ground waits (`area_waiting`), not the scenes that are only drawn. A precinct
/// seen through a portal, in another realm, waits until the viewpoint is in its realm.
fn start_builds(
    viewpoint: Res<Viewpoint>,
    wanting: Query<(Entity, &Precinct), With<WantsNavMesh>>,
    scenery: SceneryPending,
    terrain: TerrainPending,
    capture: NavInputCapture,
    mut commands: Commands,
) {
    if wanting.is_empty() {
        return;
    }
    for (entity, precinct) in &wanting {
        if viewpoint.realm != Some(precinct.realm) {
            continue;
        }
        let area = Rect::from_corners(
            precinct.coords.as_vec2() * PRECINCT_SIZE_F,
            (precinct.coords + 1).as_vec2() * PRECINCT_SIZE_F,
        );
        // The rect's far edges lie on the next parcels' near edges; pulled in so they are not
        // waited on.
        let area = area.inflate(-0.5);
        if scenery.area_waiting(precinct.realm, area).is_some()
            || terrain.area_waiting(precinct.realm, area).is_some()
        {
            continue;
        }
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WantsNavMesh>();
        match capture.capture(precinct.realm, precinct.coords) {
            Ok(input) => {
                let task =
                    AsyncComputeTaskPool::get().spawn(async move { clipping::build(&input) });
                entity_commands.insert(NavMeshBuild(task));
            }
            Err(error) => warn!("Precinct {}: no navigation input: {error}", precinct.coords),
        }
    }
}

/// Puts each finished mesh on its precinct.
fn finish_builds(mut builds: Query<(Entity, &mut NavMeshBuild)>, mut commands: Commands) {
    for (entity, mut build) in &mut builds {
        if let Some(mesh) = check_ready(&mut build.0) {
            commands
                .entity(entity)
                .remove::<NavMeshBuild>()
                .insert(PrecinctNavMesh(mesh));
        }
    }
}
