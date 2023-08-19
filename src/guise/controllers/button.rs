use bevy::{prelude::*, ui::FocusPolicy};
use bevy_mod_picking::prelude::*;

use crate::guise::{
    controller::Controller,
    style::{ComputedStyle, PartialStyle, UpdateComputedStyle},
    ViewElement,
};

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
    fn attach(&self, _commands: &Commands, _entity: Entity, _view: &ViewElement) {
        println!("Attach button");
    }

    fn update_styles(
        &self,
        commands: &mut Commands,
        entity: Entity,
        view: &ViewElement,
        assets: &Assets<PartialStyle>,
    ) {
        let mut computed = ComputedStyle::default();
        view.apply_base_styles(&mut computed, assets);

        if self.pressed {
            computed.background_color = Some(PRESSED);
        } else if self.hover {
            computed.background_color = Some(HOVERED);
            computed.border_color = Some(Color::WHITE);
        } else {
            computed.background_color = Some(NORMAL);
        }

        view.apply_inline_styles(&mut computed);
        commands.add(UpdateComputedStyle { entity, computed });
    }
}

const NORMAL: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED: Color = Color::rgb(0.35, 0.75, 0.35);

pub fn button_controller_init(
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
