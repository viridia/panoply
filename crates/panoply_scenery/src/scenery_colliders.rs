//! Physics colliders for scenery elements.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderShape {
    #[default]
    Box,
    Rbox,
    Sphere,
    Ellipsoid,
    Ramp,
    Cylinder,
}

#[derive(Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderType {
    #[default]
    Solid, // Default type
    Door,   // Openable door, affects pathfinding in game
    Ladder, // Climable ladder
    Sensor, // Generates an event when actor intersects
    Hint,   // This collider is a hint to the pathfinder, but does not actually block movement.
    Portal, // Portal volume
    Marker, // Means this collider is a proxy for picking in the editor, but has no effect in game.
}

/// Describes a physics collider on a scenery element.
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ColliderDesc {
    pub shape: ColliderShape,

    #[serde(default)]
    pub r#type: ColliderType,

    pub size: Option<Vec3>, // Note that for spheres, only y is used.
    pub offset: Option<Vec3>,

    // Direction facing (0-3) relative to wall tile.
    pub facing: Option<f32>,

    /** Whether this collider receives mouse events. */
    pub pickable: Option<bool>,

    /** If true, this is an animated collider that is attached to a sub-component of the tile. */
    pub animation: Option<String>,

    /** For animated tiles,. */
    pub origin: Option<Vec3>,

    /** Means we can walk on top of this collider. Affects pathfinding and physics. */
    pub walkable: Option<bool>,
}
