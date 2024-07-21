use bevy::{math::IRect, prelude::*};

mod viewpoint;
pub mod viewport;
pub use viewpoint::*;

/// Marker which identifies the primary camera.
#[derive(Component)]
pub struct PrimaryCamera;

/// Marker which identifies the HUD camera.
#[derive(Component)]
pub struct HudCamera;

/// Used to query precincts or parcels
pub struct QueryRect {
    pub realm: Entity,
    pub bounds: IRect,
}
