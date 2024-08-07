use bevy::prelude::*;

use crate::terrain::ShapeRef;

/// Trigger event which changes the terrain for a parcel.
#[derive(Clone, Debug, Event)]
pub struct ChangeTerrainEvent {
    pub realm: Entity,
    pub coords: IVec2,
    pub shape: ShapeRef,
}

/// Trigger event which fires when a terrain contour is changed. This causes a rebuild of
/// the thumbnail.
#[derive(Clone, Debug, Event)]
pub struct ChangeContourEvent(pub usize);

/// Trigger event which fires when the thumbnail table is ready.
#[derive(Clone, Debug, Event)]
pub struct ThumbnailsReady;
