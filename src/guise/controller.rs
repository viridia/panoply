use bevy::prelude::*;

use super::{
    style::{ComputedStyle, PartialStyle, UpdateComputedStyle},
    ViewElement,
};

/// A controller is an object which attaches to a UiComponent and handles events.
#[bevy_trait_query::queryable]
pub trait Controller {
    // TODO: This does nothing yet.
    fn attach(&self, _commands: &Commands, _entity: Entity, _view: &ViewElement) {}

    fn update_styles(
        &self,
        commands: &mut Commands,
        entity: Entity,
        view: &ViewElement,
        assets: &Assets<PartialStyle>,
    ) {
        let mut computed = ComputedStyle::default();
        view.apply_base_styles(&mut computed, assets);
        view.apply_inline_styles(&mut computed);
        commands.add(UpdateComputedStyle { entity, computed });
    }
}

// pub enum UiEvent {
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
