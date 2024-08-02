use bevy::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PickTarget {
    #[default]
    None,
    Parcel(Entity),
    Scenery,
    Actor,
    Fixture,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PickAction {
    // Move(Vec3),
    Leave,
    Down(Vec3),
    RightClick,
    DblClick,
    DragStart(Vec3),
    Drag,
    DragEnd,
}

#[derive(Clone, Debug, Event)]
pub struct PickEvent {
    pub target: PickTarget,
    pub action: PickAction,
}