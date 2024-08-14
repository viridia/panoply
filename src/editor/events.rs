use bevy::prelude::*;
use panoply_exemplar::Exemplar;

use crate::{scenery::precinct_asset::PrecinctAsset, terrain::ShapeRef};

/// Trigger event which changes the terrain for a parcel.
#[derive(Clone, Debug, Event)]
pub struct ModifyTerrainMapEvent {
    pub realm: Entity,
    pub coords: IVec2,
    pub shape: ShapeRef,
}

/// Trigger event which does a boolean operation on floors.
#[derive(Clone, Debug, Event)]
pub struct FloorStampEvent {
    pub precinct: Entity,
    pub tier: i32,
    pub floor_type: Option<AssetId<Exemplar>>,
    pub shape: Vec<Vec<Vec2>>,
}

/// Trigger event which fires when a terrain contour is changed. This causes a rebuild of
/// the thumbnail.
#[derive(Clone, Debug, Event)]
pub struct ChangeContourEvent(pub usize);

/// Trigger event which fires when the thumbnail table is ready.
#[derive(Clone, Debug, Event)]
pub struct ThumbnailsReady;

/// Rotate the current selection.
#[derive(Clone, Debug, Event)]
pub struct RotateSelection(pub i32);

#[derive(Clone, Debug, Event)]
pub struct PlaceWalls {
    pub precinct: Handle<PrecinctAsset>,
    pub tier: i16,
    pub area: Rect,
    pub facing: f32,
    pub exemplar: AssetId<Exemplar>,
    pub replace: bool,
}

#[derive(Clone, Debug, Event)]
pub struct RemoveWalls {
    pub precinct: Handle<PrecinctAsset>,
    pub tier: i16,
    pub area: Rect,
}
