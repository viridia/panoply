use bevy::prelude::*;

use super::template::TemplateNode;

// pub struct PointerEvent {
//     target: Entity,
//     pos: Vec2,
//     // buttons
// }

// pub struct FocusEvent {
//     target: Entity,
//     // value: Any,
// }

// pub struct ChangeEvent {
//     target: Entity,
//     // value: Any,
// }

// pub enum UiEvent {
//     // Pointer events.
//     PointerDown(PointerEvent),
//     PointerUp(PointerEvent),
//     PointerIn(PointerEvent),
//     PointerOut(PointerEvent),
//     PointerMove(PointerEvent),
//     PointerCancel(PointerEvent),

//     // Triggered by a pointer up event while active (not rolled off).
//     Click(PointerEvent),
//     // Wheel(PointerEvent),

//     // Fired continuously while the widget state is updating.
//     Input(ChangeEvent),

//     // Fired when widget has finished updating.
//     Change(ChangeEvent),

//     // Focus events.
//     Focus(FocusEvent),
//     Blur(FocusEvent),
// }

/// A controller is an object which attaches to a UiComponent and handles events.
pub trait Controller {
    // TODO: This does nothing yet.
    fn attach(&self, _commands: Commands, _entity: Entity, _template_node: &TemplateNode) {}
}
