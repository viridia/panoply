use crate::view::Viewpoint;
use bevy::prelude::*;
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, RoundedCorners};

#[derive(Resource, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("zoom"))]
pub struct ZoomLevel {
    #[reflect(ignore)]
    pub active: bool,
    #[reflect(ignore)]
    pub target: f32,

    #[reflect(@PreferencesKey("zoom"))]
    pub level: f32,
    #[reflect(ignore)]
    pub rate: f32,
}

impl ZoomLevel {
    const MIN_LEVEL: f32 = 0.0;
    const MAX_LEVEL: f32 = 2.0;
}

#[derive(Clone, PartialEq)]
pub(crate) struct ZoomSelector;

impl ViewTemplate for ZoomSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let zoom = cx.use_resource::<ZoomLevel>().target;

        ToolPalette::new().size(Size::Xl).columns(5).children((
            ToolIconButton::new("embedded://bevy_quill_obsidian/assets/icons/zoom_out.png")
                .corners(RoundedCorners::Left)
                .size(Vec2::new(24., 24.))
                .disabled(zoom >= ZoomLevel::MAX_LEVEL)
                .on_click(cx.create_callback(|mut zoom: ResMut<ZoomLevel>| {
                    zoom.target =
                        (zoom.target + 1.0).clamp(ZoomLevel::MIN_LEVEL, ZoomLevel::MAX_LEVEL);
                    if zoom.target != zoom.level {
                        zoom.rate = (zoom.target - zoom.level) * 3.0;
                        zoom.active = true;
                    }
                })),
            ToolIconButton::new("embedded://bevy_quill_obsidian/assets/icons/zoom_in.png")
                .corners(RoundedCorners::Right)
                .size(Vec2::new(24., 24.))
                .disabled(zoom <= ZoomLevel::MIN_LEVEL)
                .on_click(cx.create_callback(|mut zoom: ResMut<ZoomLevel>| {
                    zoom.target =
                        (zoom.target - 1.0).clamp(ZoomLevel::MIN_LEVEL, ZoomLevel::MAX_LEVEL);
                    if zoom.target != zoom.level {
                        zoom.rate = (zoom.target - zoom.level) * 3.0;
                        zoom.active = true;
                    }
                })),
        ))
    }
}

pub fn update_zoom_level(
    mut r_zoom: ResMut<ZoomLevel>,
    r_time: Res<Time>,
    mut r_viewpoint: ResMut<Viewpoint>,
) {
    if r_zoom.active {
        r_zoom.level += r_zoom.rate * r_time.delta_seconds();
        if (r_zoom.rate > 0.0 && r_zoom.level >= r_zoom.target)
            || (r_zoom.rate < 0.0 && r_zoom.level <= r_zoom.target)
        {
            r_zoom.level = r_zoom.target;
            r_zoom.active = false;
        }
    } else {
        r_zoom.target = r_zoom.level;
    }

    let distance = 2.0f32.powf(r_zoom.level) * 20.0;
    if r_viewpoint.camera_distance() != distance {
        r_viewpoint.set_camera_distance(distance);
    }
}
