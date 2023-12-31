use bevy::{math::IRect, prelude::*};

mod viewpoint;
mod viewport;
pub use viewpoint::*;
pub use viewport::*;

/// Marker which identifies the primary camera.
#[derive(Component)]
pub struct PrimaryCamera;

/// Used to query precincts or parcels
pub struct QueryRect {
    pub realm: Entity,
    pub bounds: IRect,
}
