use bevy::ui;
use bevy_mod_stylebuilder::{
    StyleBuilder, StyleBuilderBackground, StyleBuilderFont, StyleBuilderLayout,
};
use bevy_quill_obsidian::colors;

pub fn style_attribute_list(sb: &mut StyleBuilder) {
    sb.display(ui::Display::Grid)
        .grid_auto_rows(vec![ui::GridTrack::default()])
        .grid_template_columns(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ]);
}

pub fn style_attribute_key(sb: &mut StyleBuilder) {
    sb.background_color(colors::U2)
        .color(colors::DIM)
        .padding((6, 3))
        .justify_content(ui::JustifyContent::FlexEnd);
}

pub fn style_attribute_value(sb: &mut StyleBuilder) {
    sb.background_color(colors::U1)
        .color(colors::FOREGROUND)
        .padding((6, 3));
}
