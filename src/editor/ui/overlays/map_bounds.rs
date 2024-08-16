use bevy::{
    color::{palettes, Alpha},
    math::{IRect, Rect, Vec2},
    prelude::Transform,
    render::view::RenderLayers,
};
use bevy_quill::{Cx, View, ViewTemplate};
use bevy_quill_overlays::{Overlay, PolygonOptions, ShapeOrientation};

use crate::{scenery::PRECINCT_SIZE_F, terrain::PARCEL_SIZE_F, view::Viewpoint, world::Realm};

#[derive(Clone, PartialEq)]
pub struct MapBoundsOverlay;

impl ViewTemplate for MapBoundsOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let viewpoint = cx.use_resource::<Viewpoint>();
        let realm = viewpoint
            .realm
            .map(|r| cx.use_component::<Realm>(r).unwrap());
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };
        let parcel_bounds = match realm {
            Some(realm) => {
                let min = realm.parcel_bounds.min.as_vec2() * PARCEL_SIZE_F;
                let max = realm.parcel_bounds.max.as_vec2() * PARCEL_SIZE_F;
                Rect::from_corners(min, max)
            }
            None => Rect::default(),
        };
        let precinct_bounds = match realm {
            Some(realm) => realm.precinct_bounds,
            None => IRect::default(),
        };

        // println!("viewpoint: {:?}", viewpoint);
        Overlay::new()
            .named("MapBoundsOverlay")
            .shape_dyn(
                |(parcel_bounds, precinct_bounds), sb| {
                    sb.with_orientation(ShapeOrientation::YPositive)
                        .with_stroke_width(0.4)
                        .stroke_rect(parcel_bounds.inflate(0.5));
                    sb.with_stroke_width(0.05);
                    for x in precinct_bounds.min.x..precinct_bounds.max.x {
                        sb.stroke_polygon(
                            &[
                                Vec2::new(
                                    x as f32 * PRECINCT_SIZE_F,
                                    precinct_bounds.min.y as f32 * PRECINCT_SIZE_F,
                                ),
                                Vec2::new(
                                    x as f32 * PRECINCT_SIZE_F,
                                    precinct_bounds.max.y as f32 * PRECINCT_SIZE_F,
                                ),
                            ],
                            PolygonOptions::default(),
                        );
                    }
                    for y in precinct_bounds.min.y..precinct_bounds.max.y {
                        sb.stroke_polygon(
                            &[
                                Vec2::new(
                                    precinct_bounds.min.x as f32 * PRECINCT_SIZE_F,
                                    y as f32 * PRECINCT_SIZE_F,
                                ),
                                Vec2::new(
                                    precinct_bounds.max.x as f32 * PRECINCT_SIZE_F,
                                    y as f32 * PRECINCT_SIZE_F,
                                ),
                            ],
                            PolygonOptions::default(),
                        );
                    }
                },
                (parcel_bounds, precinct_bounds),
            )
            .color(palettes::css::WHITE.with_alpha(0.3))
            .insert(Transform::from_xyz(0., 0.001, 0.))
            .insert_dyn(|layer| layer, layer)
    }
}
