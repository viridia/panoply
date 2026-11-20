//! Navigation: each precinct's mesh, and what it is built from.

mod agent;
pub mod clipping;
mod input;
mod mesh;
pub mod recast;
mod remote;

use bevy::{
    app::{App, Plugin, Startup},
    reflect::Reflect,
};

pub use input::{CaptureError, NavCollider, NavInput, NavInputCapture, NavSurface, SurfaceKind};
pub use mesh::{
    NavAreaKind, NavEdge, NavMesh, NavMeshError, NavPolygon, PrecinctNavMesh, PrecinctSide,
};

/// Which generator builds a mesh, while PAN16e compares them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Reflect)]
pub enum NavGenerator {
    /// [`recast`], at its default settings.
    #[default]
    Recast,
    /// [`clipping`], the port of faery's builder.
    Clipping,
}

impl NavGenerator {
    pub fn build(self, input: &NavInput) -> Result<NavMesh, String> {
        match self {
            Self::Recast => {
                recast::build(input, &recast::RecastSettings::default()).map_err(|e| e.to_string())
            }
            Self::Clipping => Ok(clipping::build(input)),
        }
    }
}

pub struct PanoplyNavPlugin;

impl Plugin for PanoplyNavPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PrecinctNavMesh>();
        // At startup, once BRP's plugin has added its methods, as the editor's survey does.
        app.add_systems(Startup, remote::register);
    }
}
