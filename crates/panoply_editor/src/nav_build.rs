//! Asks for the loaded precincts' navigation meshes for the navigation view (`nav_view`), beyond
//! the simulated precincts the game builds: each precinct without a mesh when Navigation turns on,
//! and each that loads while it is on. `panoply_nav` builds them, and rebuilds them after edits.

use bevy::prelude::*;
use panoply_nav::{NavMeshWanted, PrecinctNavMesh};
use panoply_scenery::precinct::Precinct;

use crate::debug_views::DebugViews;

pub struct NavBuildPlugin;

impl Plugin for NavBuildPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                want_meshes.run_if(resource_changed::<DebugViews>),
                want_new_precincts,
            ),
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
            commands.entity(entity).insert(NavMeshWanted);
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
            commands.entity(entity).insert(NavMeshWanted);
        }
    }
}
