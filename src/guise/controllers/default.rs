use bevy::{prelude::*, ui::FocusPolicy};

use crate::guise::{controller::Controller, ViewElement};

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct DefaultController;

impl Controller for DefaultController {
    fn attach(&self, _commands: &Commands, _entity: Entity, _view: &ViewElement) {
        println!("Attach default");
    }
}

pub fn default_controller_init(
    mut commands: Commands,
    query: Query<(Entity, &DefaultController), Added<DefaultController>>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert((FocusPolicy::Block,));
    }
}
