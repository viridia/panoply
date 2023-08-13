use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    prelude::*,
    reflect::Reflect,
    ui::{JustifyContent, Val},
};

use crate::guise::{controller::Controller, template::TemplateNode};

#[derive(Reflect, Default)]
#[reflect(Default)]
pub struct ButtonController {}

impl Controller for ButtonController {
    fn render(&mut self, mut commands: Commands, node: &TemplateNode) -> Entity {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    ..default()
                },
                AccessibilityNode(NodeBuilder::new(Role::Button)),
            ))
            .id()
    }
}
