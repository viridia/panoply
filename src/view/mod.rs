use bevy::prelude::*;

mod viewpoint;
mod viewport;
pub use viewpoint::*;
pub use viewport::*;

#[derive(Component)]
pub struct PrimaryCamera;
