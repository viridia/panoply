use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{
    prelude::{Button, *},
    typography, RoundedCorners,
};

use super::controls::{
    style_attribute_key, style_attribute_list, style_attribute_value, LocationChooser,
};

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeRealmControls;

impl ViewTemplate for EditModeRealmControls {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let selected = cx.create_mutable::<Option<String>>(None);
        let on_select = cx.create_callback(move |key: In<String>, world: &mut World| {
            selected.set_clone(world, Some(key.clone()));
        });

        Element::<NodeBundle>::new().style(style_panel).children((
            Element::<NodeBundle>::new()
                .style((
                    typography::text_default,
                    style_attribute_list,
                    style_summary,
                ))
                .children((
                    Element::<NodeBundle>::new()
                        .style((
                            style_attribute_key,
                            RoundedCorners::TopLeft.to_border_style(4.).into_handle(),
                        ))
                        .children("Realm"),
                    Element::<NodeBundle>::new()
                        .style((
                            style_attribute_value,
                            RoundedCorners::TopRight.to_border_style(4.).into_handle(),
                        ))
                        .children("--"),
                    Element::<NodeBundle>::new()
                        .style((
                            style_attribute_key,
                            RoundedCorners::BottomLeft.to_border_style(4.).into_handle(),
                        ))
                        .children("Realm Size"),
                    Element::<NodeBundle>::new()
                        .style((
                            style_attribute_value,
                            RoundedCorners::BottomRight
                                .to_border_style(4.)
                                .into_handle(),
                        ))
                        .children("(0, 0)"),
                )),
            Flex::column(|sb| {
                sb.gap(8).flex_grow(1.).align_items(ui::AlignItems::Stretch);
            })
            .children((
                Button::new().children("Create Realm...").disabled(true),
                Button::new().children("Go To Realm..."),
            )),
            Element::<NodeBundle>::new()
                .style(style_navpoint_controls)
                .children((
                    LocationChooser {
                        selected: selected.get_clone(cx),
                        on_change: on_select,
                    },
                    Flex::row(|sb| {
                        sb.justify_content(ui::JustifyContent::FlexEnd)
                            .gap(4)
                            .align_items(ui::AlignItems::Center);
                    })
                    .children((
                        Button::new().children("+").disabled(true),
                        Button::new().children("-").disabled(true),
                        Button::new().children("Go").disabled(true),
                    )),
                )),
        ))
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

fn style_summary(ss: &mut StyleBuilder) {
    ss.flex_grow(1.).grid_column_start(1).grid_column_span(2);
}

fn style_navpoint_controls(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Flex)
        .flex_direction(ui::FlexDirection::Column)
        .gap(8)
        .flex_grow(1.)
        .align_items(ui::AlignItems::Stretch);
}
