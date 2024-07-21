use bevy::{prelude::*, ui};
use bevy_mod_stylebuilder::*;
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{
    colors,
    controls::{Splitter, SplitterDirection},
    focus::TabGroup,
    viewport::ViewportInsetElement,
};

use crate::view::HudCamera;

use super::EditorSidebarWidth;

pub fn setup_editor_view(mut commands: Commands, q_camera: Query<Entity, With<HudCamera>>) {
    let camera = q_camera.get_single().expect("HudCamera not found");
    commands.spawn(EditorView(camera).to_root());
}

#[derive(Clone, PartialEq)]
struct EditorView(Entity);

impl ViewTemplate for EditorView {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        // Needed to ensure popup menus and dialogs render on the correct camera.
        let camera = self.0;
        cx.insert(TargetCamera(camera));

        let sidebar_width = cx.use_resource::<EditorSidebarWidth>();
        Element::<NodeBundle>::new()
            .insert((TabGroup::default(), TargetCamera(camera)))
            .style(style_main)
            .children((
                Element::<NodeBundle>::new().style(style_aside).style_dyn(
                    move |width, sb| {
                        sb.width(ui::Val::Px(width));
                    },
                    sidebar_width.0,
                ),
                Splitter::new()
                    .direction(SplitterDirection::Vertical)
                    .value(sidebar_width.0)
                    .on_change(cx.create_callback(|value: In<f32>, world: &mut World| {
                        let mut panel_width =
                            world.get_resource_mut::<EditorSidebarWidth>().unwrap();
                        panel_width.0 = value.max(200.);
                    })),
                Element::<NodeBundle>::new()
                    .style(style_game_view)
                    .insert(ViewportInsetElement),
            ))
    }
}

fn style_main(ss: &mut StyleBuilder) {
    ss.position(ui::PositionType::Absolute)
        .left(0)
        .top(0)
        .bottom(0)
        .right(0)
        .border(1)
        .border_color(colors::U2)
        .display(ui::Display::Flex)
        .pointer_events(false);
}

fn style_aside(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Flex)
        .background_color(colors::U2)
        .padding(8)
        .gap(8)
        .flex_direction(ui::FlexDirection::Column)
        .width(200)
        .pointer_events(true);
}

fn style_game_view(ss: &mut StyleBuilder) {
    ss.display(Display::Flex)
        .flex_grow(1.)
        .align_self(ui::AlignSelf::Stretch)
        .flex_direction(FlexDirection::Column);
}
