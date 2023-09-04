use bevy::{prelude::*, ui::FocusPolicy};
use bevy_mod_picking::prelude::*;

use crate::guise::{
    controller::Controller,
    style::{ComputedStyle, StyleAsset, UpdateComputedStyle},
    ViewElement,
};

#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct SliderController {
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub disabled: bool,
    pub dragging: bool,
    pub inside: bool,
}

impl FromWorld for SliderController {
    fn from_world(_world: &mut World) -> Self {
        SliderController {
            min: 0.,
            max: 1.,
            value: 0.,
            disabled: false,
            dragging: false,
            inside: false,
        }
    }
}

impl Controller for SliderController {
    fn attach(&self, commands: &mut Commands, entity: Entity, _view: &ViewElement) {
        // REINSTATE
        // commands.entity(entity).insert((
        //     On::<Pointer<Over>>::run(button_pointer_over),
        //     On::<Pointer<Out>>::run(button_pointer_out),
        //     On::<Pointer<DragStart>>::run(button_drag_start),
        //     On::<Pointer<DragEnd>>::run(button_drag_end),
        //     FocusPolicy::Block,
        // ));
    }

    fn update_styles(
        &self,
        commands: &mut Commands,
        entity: Entity,
        view: &ViewElement,
        assets: &Assets<StyleAsset>,
    ) {
        let mut computed = ComputedStyle::default();

        let mut classes: Vec<&str> = Vec::with_capacity(3);
        if self.disabled {
            classes.push("disabled");
        } else if self.inside {
            if self.dragging {
                classes.push("pressed");
            } else {
                classes.push("hover");
            }
        }

        self.compute_style(&mut computed, view, assets);
        commands.add(UpdateComputedStyle { entity, computed });
    }
}

const NORMAL: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED: Color = Color::rgb(0.25, 0.25, 0.35);
const PRESSED: Color = Color::rgb(0.35, 0.75, 0.35);

fn button_pointer_over(
    _event: Listener<Pointer<Over>>,
    mut _query: Query<(&mut ViewElement, &mut SliderController)>,
) {
    // if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
    //     if !ctrl.disabled {
    //         ctrl.inside = true;
    //         view.set_changed();
    //     }
    // }
}

fn button_pointer_out(
    _event: Listener<Pointer<Out>>,
    mut _query: Query<(&mut ViewElement, &mut SliderController)>,
) {
    // if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
    //     ctrl.inside = false;
    //     view.set_changed();
    // }
}

fn button_drag_start(
    _event: Listener<Pointer<DragStart>>,
    mut _query: Query<(&mut ViewElement, &mut SliderController)>,
) {
    // if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
    //     ctrl.dragging = true;
    //     view.set_changed();
    // }
}

fn button_drag_end(
    _event: Listener<Pointer<DragEnd>>,
    mut _query: Query<(&mut ViewElement, &mut SliderController)>,
) {
    // if let Ok((mut view, mut ctrl)) = query.get_mut(event.listener()) {
    //     ctrl.dragging = false;
    //     view.set_changed();
    // }
}
