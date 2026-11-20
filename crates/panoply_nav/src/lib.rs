//! Navigation: each precinct's mesh, and what it is built from.

mod agent;
pub mod clipping;
mod input;
mod mesh;
mod remote;

use bevy::app::{App, Plugin, Startup};

pub use input::{CaptureError, NavCollider, NavInput, NavInputCapture, NavSurface, SurfaceKind};
pub use mesh::{
    NavAreaKind, NavEdge, NavMesh, NavMeshError, NavPolygon, PrecinctNavMesh, PrecinctSide,
};

pub struct PanoplyNavPlugin;

impl Plugin for PanoplyNavPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PrecinctNavMesh>();
        // At startup, once BRP's plugin has added its methods, as the editor's survey does.
        app.add_systems(Startup, remote::register);
    }
}
