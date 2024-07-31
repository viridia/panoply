use bevy::prelude::*;

#[derive(Clone, Debug)]
pub enum PickTarget {
    Parcel(Entity),
    Scenery,
    Actor,
    Fixture,
}

#[derive(Clone, Debug)]
pub struct Pick {
    // TODO: world pos
    pub target: PickTarget,
}

#[derive(Clone, Debug, Event)]
pub enum PickEvent {
    Move(Pick),
    BeginStroke(Pick),
    ContinueStroke(Pick),
    EndStroke(Pick),
    RightClick(Pick),
    DblClick(Pick),
}
