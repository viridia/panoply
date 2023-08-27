use bevy::ecs::system::Command;
use bevy::prelude::*;
use bevy::text::BreakLineOn;

use super::color::ColorValue;

/// A computed style represents the composition of one or more `PartialStyle`s.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct ComputedStyle {
    pub style: Style,

    // Text properties
    pub alignment: Option<TextAlignment>,
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub font: Option<Handle<Font>>,
    pub line_break: Option<BreakLineOn>,

    // pub text_style: TextStyle,
    pub border_color: Option<Color>,
    pub background_color: ColorValue,
    pub z_index: Option<i32>,
}

impl ComputedStyle {
    /// Construct a new, default style
    pub fn new() -> Self {
        Self { ..default() }
    }

    /// Construct a new style that inherits from a parent style. Only attributes which
    /// are inheritable will be inherited, all others will be set to the default.
    pub fn inherit(parent: &Self) -> Self {
        Self {
            alignment: parent.alignment,
            color: parent.color,
            font_size: parent.font_size,
            font: parent.font.clone(),
            line_break: parent.line_break.clone(),
            ..default()
        }
    }
}

/// Custom command that updates the style of an entity.
pub struct UpdateComputedStyle {
    pub(crate) entity: Entity,
    pub(crate) computed: ComputedStyle,
}

impl Command for UpdateComputedStyle {
    // TODO: This should probably walk the tree of children.
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

            match e.get_mut::<Text>() {
                Some(mut text) => {
                    if let Some(color) = self.computed.color {
                        for section in text.sections.iter_mut() {
                            section.style.color = color;
                        }
                    }

                    if let Some(ws) = self.computed.line_break {
                        if text.linebreak_behavior != ws {
                            text.linebreak_behavior = ws;
                        }
                    }
                }

                None => {}
            }

            match e.get_mut::<BackgroundColor>() {
                Some(mut bg_comp) => {
                    if self.computed.background_color.is_transparent() {
                        // Remove the background
                        e.remove::<BackgroundColor>();
                    } else {
                        let color = self.computed.background_color.color();
                        // Mutate the background
                        if bg_comp.0 != color {
                            bg_comp.0 = color
                        }
                    }
                }

                None => {
                    if !self.computed.background_color.is_transparent() {
                        // Insert a new background
                        e.insert(BackgroundColor(self.computed.background_color.color()));
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
