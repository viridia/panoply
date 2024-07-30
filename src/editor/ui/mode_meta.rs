use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
// use bevy_quill_obsidian::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeMetadataControls;

impl ViewTemplate for EditModeMetadataControls {
    type View = impl View;

    fn create(&self, _cx: &mut Cx) -> Self::View {
        Element::<NodeBundle>::new()
            .style(style_panel)
            .children("Metadata Edit Controls")
    }
}

fn style_panel(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Grid)
        .grid_template_columns(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .grid_template_rows(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .gap(8)
        .flex_grow(1.);
}
