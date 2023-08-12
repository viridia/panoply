use bevy::prelude::*;

pub struct PointerEvent {
    target: Entity,
    pos: Vec2,
    // buttons
}

pub struct FocusEvent {
    target: Entity,
    // value: Any,
}

pub struct ChangeEvent {
    target: Entity,
    // value: Any,
}

pub enum UiEvent {
    PointerDown(PointerEvent),
    PointerUp(PointerEvent),
    PointerIn(PointerEvent),
    PointerOut(PointerEvent),
    PointerMove(PointerEvent),
    PointerCancel(PointerEvent),

    Change(ChangeEvent),

    Focus(FocusEvent),
    Blur(FocusEvent),
}

/// A controller is an object which attaches to a UiComponent and handles events.
pub trait Controller {
    fn init(&self, commands: Commands);
    fn cleanup(&self, commands: Commands);
    fn on_event(&self, commands: Commands, entity: Entity, event: &UiEvent);
}
