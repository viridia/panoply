use bevy::{
    color::{palettes, Alpha},
    math::{Rect, Vec2, Vec3},
    prelude::{Entity, Transform},
    render::view::RenderLayers,
};
use bevy_quill::{Cond, Cx, View, ViewTemplate};
use bevy_quill_overlays::{LinesBuilder, Overlay, ShapeOrientation};

use crate::{
    editor::ui::mode_scenery::SelectedPrecinct,
    scenery::{precinct::Precinct, PRECINCT_SIZE, PRECINCT_SIZE_F},
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct SelectedPrecinctOverlay;

impl ViewTemplate for SelectedPrecinctOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let precinct = cx.use_resource::<SelectedPrecinct>().0;
        Cond::new(precinct.is_some(), SelectedPrecinctGrid { precinct }, ())
    }
}

#[derive(Clone, PartialEq)]
pub struct SelectedPrecinctGrid {
    pub precinct: Option<Entity>,
}

impl ViewTemplate for SelectedPrecinctGrid {
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

        // The bounds of the parcel in world space.
        let precinct_bounds = {
            let min = precinct.coords.as_vec2() * PRECINCT_SIZE_F;
            Rect::from_corners(min, min + Vec2::splat(PRECINCT_SIZE_F))
        };

        let height = 0.01;
        (
            Overlay::new()
                .mesh_dyn(
                    |_, sb: &mut LinesBuilder| {
                        for x in 0..=PRECINCT_SIZE {
                            sb.line(
                                Vec3::new(x as f32, 0., 0.),
                                Vec3::new(x as f32, 0., PRECINCT_SIZE_F),
                            );
                            sb.line(
                                Vec3::new(0., 0., x as f32),
                                Vec3::new(PRECINCT_SIZE_F, 0., x as f32),
                            );
                        }
                    },
                    (),
                )
                .color(palettes::css::DODGER_BLUE.with_alpha(0.9))
                .transform(Transform::from_translation(Vec3::new(
                    precinct_bounds.min.x,
                    height,
                    precinct_bounds.min.y,
                )))
                .insert_dyn(|layer| layer, layer.clone()),
            Overlay::new()
                .shape_dyn(
                    |_, sb| {
                        sb.with_stroke_width(0.2)
                            .with_orientation(ShapeOrientation::YPositive);
                        sb.stroke_rect(Rect::from_corners(
                            Vec2::ZERO,
                            Vec2::splat(PRECINCT_SIZE_F),
                        ));
                    },
                    (),
                )
                .color(palettes::css::DODGER_BLUE.with_alpha(0.9))
                .transform(Transform::from_translation(Vec3::new(
                    precinct_bounds.min.x,
                    height,
                    precinct_bounds.min.y,
                )))
                .insert_dyn(|layer| layer, layer),
        )
    }
}
