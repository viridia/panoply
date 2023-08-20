use bevy::{prelude::*, ui::FocusPolicy};

use crate::guise::controller::Controller;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct DefaultController;

impl Controller for DefaultController {}

pub fn default_controller_init(
    mut commands: Commands,
    query: Query<(Entity, &DefaultController), Added<DefaultController>>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert((FocusPolicy::Block,));
    }
}
