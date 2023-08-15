use bevy::prelude::*;

/// A computed style represents the composition of one or more `PartialStyle`s.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct ComputedStyle {
    pub style: Style,
    pub border_color: Option<Color>,
    pub background_color: Option<Color>,
    pub z_index: Option<i32>,
}
