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
    fn from_world(_world: &mut World) -> Self {
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
            On::<Pointer<Over>>::target_component_mut::<ButtonController>(|_, ctrl| {
                ctrl.hover = true;
            }),
            On::<Pointer<Out>>::target_component_mut::<ButtonController>(|_, ctrl| {
                ctrl.hover = false;
            }),
            On::<Pointer<Down>>::target_component_mut::<ButtonController>(|_, ctrl| {
                ctrl.pressed = true;
            }),
            On::<Pointer<Up>>::target_component_mut::<ButtonController>(|_, ctrl| {
                ctrl.pressed = false;
            }),
            // On::<PointerCancel>::target_component_mut::<ButtonController>(|_, ctrl| {
            //     ctrl.pressed = false;
            // }),
            FocusPolicy::Block,
        ));
    }
}

pub fn update_button(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &ButtonController,
            &mut Style,
            Option<&mut BackgroundColor>,
            Option<&mut BorderColor>,
        ),
        Changed<ButtonController>,
    >,
) {
    for (entity, ctrl, style, mut bg, border) in query.iter_mut() {
        if ctrl.pressed {
            commands
                .entity(entity)
                .insert(BackgroundColor::from(PRESSED));
        } else if ctrl.hover {
            commands
                .entity(entity)
                .insert(BackgroundColor::from(HOVERED));
        } else {
            commands
                .entity(entity)
                .insert(BackgroundColor::from(NORMAL));
        }
    }
}
