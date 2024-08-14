use bevy::{
    color::{palettes, Alpha},
    prelude::*,
    render::view::RenderLayers,
};
use bevy_quill::prelude::*;
use bevy_quill_overlays::{Overlay, ShapeOrientation};

use crate::{
    editor::ui::mode_scenery::{SceneryDragState, SelectedTier},
    scenery::{precinct::Precinct, PRECINCT_SIZE},
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct WallDrawOverlay;

impl ViewTemplate for WallDrawOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let drag_state = cx.use_resource::<SceneryDragState>();
        let rect = Rect::from_corners(drag_state.cursor_pos, drag_state.anchor_pos).inflate(0.5);
        (Cond::new(
            drag_state.precinct.is_some() && rect.width() * rect.height() > 0.0,
            WallOutline {
                precinct: drag_state.precinct,
                rect,
            },
            (),
        ),)
    }
}

#[derive(Clone, PartialEq)]
pub struct WallOutline {
    pub precinct: Option<Entity>,
    pub rect: Rect,
}

impl ViewTemplate for WallOutline {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        // Get the parcel component and realm component
        let precinct = cx
            .use_component::<Precinct>(self.precinct.unwrap())
            .unwrap();
        let realm = cx.use_component::<Realm>(precinct.realm);
        let tier = cx.use_resource::<SelectedTier>().0;

        // We need the render layers from the realm.
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };

        let height = tier as f32 + 0.011;
        Overlay::new()
            .named("WallDrawOverlay")
            .shape_dyn(
                |rect, sb| {
                    sb.with_orientation(ShapeOrientation::YPositive)
                        .with_stroke_width(0.1);
                    sb.stroke_rect(rect);
                },
                self.rect,
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
