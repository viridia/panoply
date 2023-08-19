use bevy::ecs::system::Command;
use bevy::prelude::*;

/// A computed style represents the composition of one or more `PartialStyle`s.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct ComputedStyle {
    pub style: Style,
    pub border_color: Option<Color>,
    pub background_color: Option<Color>,
    pub z_index: Option<i32>,
}

/// Custom command that updates the style of an entity.
pub struct UpdateComputedStyle {
    pub(crate) entity: Entity,
    pub(crate) computed: ComputedStyle,
}

impl Command for UpdateComputedStyle {
    fn apply(self, world: &mut World) {
        if let Some(mut e) = world.get_entity_mut(self.entity) {
            if let Some(mut style) = e.get_mut::<Style>() {
                // Update the existing style
                if !style.eq(&self.computed.style) {
                    *style = self.computed.style;
                }
            } else {
                // Insert a new style component
                e.insert(self.computed.style);
            }

            match e.get_mut::<BackgroundColor>() {
                Some(mut bg_comp) => {
                    if let Some(bg_computed) = self.computed.background_color {
                        // Mutate the background
                        if bg_comp.0 != bg_computed {
                            bg_comp.0 = bg_computed
                        }
                    } else {
                        // Remove the background
                        e.remove::<BackgroundColor>();
                    }
                }

                None => {
                    if let Some(bg_comp) = self.computed.background_color {
                        // Insert a new background
                        e.insert(BackgroundColor(bg_comp));
                    }
                }
            }

            match e.get_mut::<BorderColor>() {
                Some(mut bc_comp) => {
                    if let Some(bc_computed) = self.computed.border_color {
                        // Mutate the border color
                        if bc_comp.0 != bc_computed {
                            bc_comp.0 = bc_computed
                        }
                    } else {
                        // Remove the border color
                        e.remove::<BorderColor>();
                    }
                }

                None => {
                    if let Some(bc_comp) = self.computed.border_color {
                        // Insert a new background color
                        e.insert(BorderColor(bc_comp));
                    }
                }
            }
        }
    }
}
