use bevy::{math::IRect, prelude::*};

pub mod picking;
pub mod viewport;

/// Used to query precincts or parcels
#[derive(Debug)]
pub struct QueryRect {
    pub realm: Entity,
    pub bounds: IRect,
}
