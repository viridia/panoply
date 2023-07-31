use bevy::prelude::*;
#[derive(Eq, PartialEq, Hash)]
pub struct ParcelKey {
    pub realm: Entity,
    pub x: i32,
    pub z: i32,
}

pub const ADJACENT_COUNT: usize = 9;

// A reference to a terrain shape
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct ShapeRef {
    pub shape: u16,
    pub rotation: u8,
}

impl ShapeRef {
    pub fn new() -> ShapeRef {
        Self {
            shape: 0,
            rotation: 0,
        }
    }
}

#[derive(Component)]
pub struct Parcel {
    pub realm: Entity,
    pub coords: IVec2,
    pub visible: bool,
    pub shapes: [ShapeRef; ADJACENT_COUNT],
    pub water_entity: Option<Entity>,
}

#[derive(Component)]
pub struct ParcelContourChanged;

#[derive(Component)]
pub struct ParcelWaterChanged;

#[derive(Component)]
pub struct ParcelFloraChanged;

#[derive(Bundle)]
pub struct ParcelBundle {
    pub parcel: Parcel,
    pub mesh: PbrBundle,
}
