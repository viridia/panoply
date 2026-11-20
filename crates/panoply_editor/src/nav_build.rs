//! Builds the loaded precincts' navigation meshes for the navigation view (`nav_view`), with the
//! generator the Show menu chooses: each precinct without a mesh when Navigation turns on, and each
//! that loads while it is on, once the precinct and its parcels are ready. Built once: an edit does
//! not rebuild a mesh, and a mesh removed stays removed, until another generator is chosen, which
//! builds every precinct's again.

use bevy::{
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};
use panoply_core::Viewpoint;
use panoply_nav::{NavGenerator, NavInputCapture, NavMesh, PrecinctNavMesh};
use panoply_scenery::{precinct::Precinct, SceneryPending, PRECINCT_SIZE_F};
use panoply_terrain::TerrainPending;

use crate::debug_views::DebugViews;

/// A precinct waiting to be ready for its mesh to be built.
#[derive(Component)]
struct WantsNavMesh;

/// A precinct's mesh being built.
#[derive(Component)]
struct NavMeshBuild(Task<Result<NavMesh, String>>);

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

/// When Navigation turns on, every loaded precinct without a mesh wants one; while it is on with a
/// generator other than the one the meshes were built with, every precinct wants a new one, and
/// the builds under way are dropped.
fn want_meshes(
    views: Res<DebugViews>,
    mut was_on: Local<bool>,
    mut built_with: Local<Option<NavGenerator>>,
    precincts: Query<(Entity, Has<PrecinctNavMesh>), With<Precinct>>,
    mut commands: Commands,
) {
    let turned_on = views.navigation && !*was_on;
    *was_on = views.navigation;
    if !views.navigation {
        return;
    }
    let rebuild = built_with.is_some_and(|g| g != views.nav_generator);
    *built_with = Some(views.nav_generator);
    for (entity, has_mesh) in &precincts {
        if rebuild {
            commands
                .entity(entity)
                .remove::<(PrecinctNavMesh, NavMeshBuild)>()
                .insert(WantsNavMesh);
        } else if turned_on && !has_mesh {
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
    views: Res<DebugViews>,
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
                let generator = views.nav_generator;
                let task =
                    AsyncComputeTaskPool::get().spawn(async move { generator.build(&input) });
                entity_commands.insert(NavMeshBuild(task));
            }
            Err(error) => warn!("Precinct {}: no navigation input: {error}", precinct.coords),
        }
    }
}

/// Puts each finished mesh on its precinct.
fn finish_builds(
    mut builds: Query<(Entity, &Precinct, &mut NavMeshBuild)>,
    mut commands: Commands,
) {
    for (entity, precinct, mut build) in &mut builds {
        let Some(result) = check_ready(&mut build.0) else {
            continue;
        };
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<NavMeshBuild>();
        match result {
            Ok(mesh) => {
                entity_commands.insert(PrecinctNavMesh(mesh));
            }
            Err(error) => warn!("Precinct {}: {error}", precinct.coords),
        }
    }
}
