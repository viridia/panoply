use bevy::{prelude::*, ui::FocusPolicy};
use bevy_mod_picking::prelude::*;

use crate::guise::{
    controller::Controller,
    style::{ComputedStyle, PartialStyle, UpdateComputedStyle},
    view::StyleHandlesChanged,
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
        // println!("New ButtonController");
        ButtonController {
            pressed: false,
            hover: false,
        }
    }
}

impl Controller for ButtonController {
    fn attach(&self, commands: &mut Commands, entity: Entity, _view: &ViewElement) {
        commands.entity(entity).insert((
            On::<Pointer<Over>>::run(button_pointer_over),
            On::<Pointer<Out>>::run(button_pointer_out),
            On::<Pointer<Down>>::run(button_pointer_down),
            On::<Pointer<Up>>::run(button_pointer_up),
            // On::<PointerCancel>::listener_component_mut::<ButtonController>(|_, ctrl| {
            //     ctrl.pressed = false;
            // }),
            FocusPolicy::Block,
        ));

        // println!("Attach button");
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
const HOVERED: Color = Color::rgb(0.25, 0.25, 0.35);
const PRESSED: Color = Color::rgb(0.35, 0.75, 0.35);

fn button_pointer_over(
    event: Listener<Pointer<Over>>,
    mut commands: Commands,
    mut query: Query<(&mut ViewElement, &mut ButtonController)>,
) {
    if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
        // info!("Button hover");
        ctrl.hover = true;
        commands
            .entity(event.listener())
            .insert(StyleHandlesChanged);
        view.set_changed();
    }
}

fn button_pointer_out(
    event: Listener<Pointer<Out>>,
    mut commands: Commands,
    mut query: Query<(&mut ViewElement, &mut ButtonController)>,
) {
    if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
        ctrl.hover = false;
        commands
            .entity(event.listener())
            .insert(StyleHandlesChanged);
        view.set_changed();
    }
}

fn button_pointer_down(
    event: Listener<Pointer<Down>>,
    mut commands: Commands,
    mut query: Query<(&mut ViewElement, &mut ButtonController)>,
) {
    if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
        ctrl.pressed = true;
        commands
            .entity(event.listener())
            .insert(StyleHandlesChanged);
        view.set_changed();
    }
}

fn button_pointer_up(
    event: Listener<Pointer<Up>>,
    mut commands: Commands,
    mut query: Query<(&mut ViewElement, &mut ButtonController)>,
) {
    if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
        ctrl.pressed = false;
        commands
            .entity(event.listener())
            .insert(StyleHandlesChanged);
        view.set_changed();
    }
}
