use bevy::{
    color::{palettes, Alpha},
    math::Vec2,
    prelude::*,
    render::view::RenderLayers,
};
use bevy_quill::{Cond, Cx, View, ViewTemplate};
use bevy_quill_overlays::{Overlay, PolygonOptions, ShapeOrientation};

use crate::{
    editor::ui::mode_scenery::SceneryDragState,
    scenery::{precinct::Precinct, PRECINCT_SIZE},
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct FloorStampOverlay;

impl ViewTemplate for FloorStampOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let drag_state = cx.use_resource::<SceneryDragState>();
        Cond::new(
            drag_state.precinct.is_some(),
            PolygonCursor {
                precinct: drag_state.precinct,
                outline: drag_state.floor_outline.clone(),
            },
            (),
        )
    }
}

#[derive(Clone, PartialEq)]
pub struct PolygonCursor {
    pub precinct: Option<Entity>,
    pub outline: Vec<Vec2>,
}

impl ViewTemplate for PolygonCursor {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        // Get the parcel component and realm component
        let precinct = cx
            .use_component::<Precinct>(self.precinct.unwrap())
            .unwrap();
        let realm = cx.use_component::<Realm>(precinct.realm);

        // We need the render layers from the realm.
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };

        let height = 0.01;
        Overlay::new()
            .shape_dyn(
                |outline, sb| {
                    sb.with_orientation(ShapeOrientation::YPositive)
                        .with_stroke_width(0.1);
                    sb.stroke_polygon(
                        &outline,
                        PolygonOptions {
                            closed: true,
                            ..Default::default()
                        },
                    );
                },
                self.outline.clone(),
            )
            .color(palettes::css::SILVER.with_alpha(0.9))
            .underlay(0.8)
            .transform(Transform::from_translation(Vec3::new(
                (precinct.coords.x * PRECINCT_SIZE) as f32,
                height,
                (precinct.coords.y * PRECINCT_SIZE) as f32,
            )))
            .insert_dyn(|layer| layer, layer)
    }
}
