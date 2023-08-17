use bevy::{prelude::*, ui::FocusPolicy};
use bevy_mod_picking::prelude::*;

use crate::guise::controller::Controller;

#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct ButtonController {
    pub pressed: bool,
    pub hover: bool,
}

impl FromWorld for ButtonController {
    fn from_world(world: &mut World) -> Self {
        // let server = world.resource::<AssetServer>();
        // BiomesHandle(server.load("terrain/terrain.biomes.json"))
        println!("New ButtonController");
        ButtonController {
            pressed: false,
            hover: false,
        }
    }
}

impl Controller for ButtonController {
    fn attach(
        &self,
        _commands: Commands,
        _entity: Entity,
        _template_node: &crate::guise::template::TemplateNode,
    ) {
    }
}

const NORMAL: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED: Color = Color::rgb(0.35, 0.75, 0.35);

pub fn init_button(
    mut commands: Commands,
    query: Query<(Entity, &ButtonController), Added<ButtonController>>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert((
            On::<Pointer<Over>>::run(|| println!("Over!")),
            On::<Pointer<Out>>::listener_insert(BackgroundColor::from(NORMAL)),
            On::<Pointer<Down>>::listener_insert(BackgroundColor::from(PRESSED)),
            On::<Pointer<Up>>::listener_insert(BackgroundColor::from(HOVERED)),
            FocusPolicy::Block,
        ));
    }
}
