use super::precinct_asset::PrecinctAsset;
use bevy::prelude::*;

#[derive(Eq, PartialEq, Hash)]
pub struct PrecinctKey {
    pub realm: Entity,
    pub x: i32,
    pub z: i32,
}

#[derive(Component)]
pub struct Precinct {
    pub realm: Entity,
    pub coords: IVec2,
    pub visible: bool,
    pub asset: Handle<PrecinctAsset>,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PrecinctAssetChanged;

// #[derive(Component)]
// #[component(storage = "SparseSet")]
// pub struct ParcelWaterChanged;

// #[derive(Component)]
// #[component(storage = "SparseSet")]
// pub struct ParcelFloraChanged;
