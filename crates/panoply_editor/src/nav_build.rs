//! Builds the loaded precincts' navigation meshes for the navigation view (`nav_view`), with
//! Recast: each precinct without a mesh when Navigation turns on, and each that loads while it is
//! on, once the precinct and its parcels are ready. Built once: an edit does not rebuild a mesh,
//! and a mesh removed stays removed.

use bevy::{
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};
use panoply_nav::{
    recast::{self, RecastError, RecastSettings},
    NavInputCapture, NavMesh, PrecinctNavMesh,
};
use panoply_scenery::{precinct::Precinct, SceneryPending, PRECINCT_SIZE};
use panoply_terrain::{TerrainPending, PARCEL_SIZE};

use crate::debug_views::DebugViews;

/// A precinct waiting to be ready for its mesh to be built.
#[derive(Component)]
struct WantsNavMesh;

/// A precinct's mesh being built.
#[derive(Component)]
struct NavMeshBuild(Task<Result<NavMesh, RecastError>>);

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

/// Captures the input of each wanting precinct whose scenery and parcels are ready, and builds its
/// mesh off the main thread.
fn start_builds(
    wanting: Query<(Entity, &Precinct), With<WantsNavMesh>>,
    scenery: SceneryPending,
    terrain: TerrainPending,
    capture: NavInputCapture,
    mut commands: Commands,
) {
    if wanting.is_empty() {
        return;
    }
    let precincts = scenery.survey();
    let parcels = terrain.survey();
    for (entity, precinct) in &wanting {
        let ready = precincts.iter().any(|p| {
            p.realm == precinct.realm && p.coords == precinct.coords && p.pending.is_empty()
        });
        let side = PRECINCT_SIZE / PARCEL_SIZE;
        let first = precinct.coords * side;
        let parcels_ready = parcels
            .iter()
            .filter(|p| {
                p.realm == precinct.realm
                    && p.pending.is_empty()
                    && (first.x..first.x + side).contains(&p.coords.x)
                    && (first.y..first.y + side).contains(&p.coords.y)
            })
            .count()
            == (side * side) as usize;
        if !(ready && parcels_ready) {
            continue;
        }
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<WantsNavMesh>();
        match capture.capture(precinct.realm, precinct.coords) {
            Ok(input) => {
                let task = AsyncComputeTaskPool::get()
                    .spawn(async move { recast::build(&input, &RecastSettings::default()) });
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
