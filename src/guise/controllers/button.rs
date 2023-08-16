use bevy::prelude::*;

#[derive(Reflect, Default, Component)]
#[reflect(Default)]
#[reflect(Component)]
pub struct ButtonController {}

// impl Controller for ButtonController {
//     fn render(&mut self, mut commands: Commands, _node: &TemplateNode) -> Entity {
//         commands
//             .spawn((
//                 NodeBundle {
//                     style: Style {
//                         width: Val::Percent(100.0),
//                         height: Val::Percent(100.0),
//                         justify_content: JustifyContent::SpaceBetween,
//                         ..default()
//                     },
//                     ..default()
//                 },
//                 AccessibilityNode(NodeBuilder::new(Role::Button)),
//             ))
//             .id()
//     }
// }
