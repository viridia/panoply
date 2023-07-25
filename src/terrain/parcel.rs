use bevy::prelude::*;
#[derive(Eq, PartialEq, Hash)]
pub struct ParcelKey {
    pub realm: i32,
    pub x: i32,
    pub z: i32,
}

pub enum ParcelStatus {
    New,
    // Loading,
    Waiting,
    Building,
    Ready,
}

#[derive(Component)]
pub struct Parcel {
    pub realm: i32,
    pub coords: IVec2,
    pub status: ParcelStatus,
    pub visible: bool,
    // pub adjacent_plots: [TerrainShape; 9],
}

#[derive(Bundle)]
pub struct ParcelBundle {
    pub parcel: Parcel,
    pub mesh: PbrBundle,
}
