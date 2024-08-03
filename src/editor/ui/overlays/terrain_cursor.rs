use bevy::{
    asset::Assets,
    color::{palettes, Alpha},
    math::{Rect, Vec2},
    prelude::*,
    render::view::RenderLayers,
};
use bevy_quill::{Cx, Dynamic, IntoViewChild, View, ViewTemplate};
use bevy_quill_overlays::{Overlay, PolygonOptions, ShapeOrientation};

use crate::{
    editor::{DragShape, SelectedParcel, TerrainDragState},
    terrain::{
        rotator,
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel, PARCEL_HEIGHT_SCALE, PARCEL_SIZE, PARCEL_SIZE_U,
    },
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct TerrainCursorOverlay;

impl ViewTemplate for TerrainCursorOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        // let tool = *cx.use_resource::<State<TerrainTool>>().get();
        let drag_state = match cx.use_resource::<SelectedParcel>().0 {
            Some(_) => cx.use_resource::<TerrainDragState>().clone(),
            None => TerrainDragState::default(),
        };

        Dynamic::new(match drag_state.drag_shape {
            DragShape::None => ().into_view_child(),
            DragShape::Point => PointCursor {
                parcel: drag_state.parcel.unwrap(),
                point: drag_state.cursor_pos,
            }
            .into_view_child(),
            DragShape::FlatRect => FlatRectCursor {
                parcel: drag_state.parcel.unwrap(),
                rect: IRect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos),
                height: drag_state.anchor_height as f32 * PARCEL_HEIGHT_SCALE,
            }
            .into_view_child(),
            DragShape::DecalRect => DecalRectCursor {
                parcel: drag_state.parcel.unwrap(),
                rect: {
                    let mut rect =
                        IRect::from_corners(drag_state.anchor_pos, drag_state.cursor_pos);
                    rect.max += IVec2::splat(1);
                    rect
                },
            }
            .into_view_child(),
        })
    }
}

#[derive(Clone, PartialEq)]
pub struct PointCursor {
    pub parcel: Entity,
    pub point: IVec2,
}

impl ViewTemplate for PointCursor {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        // Get the parcel component and realm component
        let parcel = cx.use_component::<Parcel>(self.parcel).unwrap();
        let realm = cx.use_component::<Realm>(parcel.realm);

        // We need the render layers from the realm.
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };

        // Look up the parcel's terrain contours. The terrain contour table is an asset.
        // Note that assets is untracked because it's constantly changing.
        let shape_ref = parcel.center_shape();
        let ts_handle = cx.use_resource::<TerrainContoursHandle>().0.clone();
        let ts_assets = cx.use_resource_untracked::<Assets<TerrainContoursTableAsset>>();

        // Extract out the height map.
        let cursor_height = ts_assets
            .get(&ts_handle)
            .map(|contours| {
                let lock = contours.0.read().unwrap();
                let pos = self.point;
                lock.get(shape_ref.shape as usize).height_at(
                    pos.x.clamp(0, PARCEL_SIZE_U as i32) as usize,
                    pos.y.clamp(0, PARCEL_SIZE_U as i32) as usize,
                    shape_ref.rotation,
                )
            })
            .unwrap_or(0.);

        Overlay::new()
            .shape_dyn(
                |pos, sb| {
                    let rect = Rect::from_center_size(pos.as_vec2(), Vec2::splat(0.35));
                    sb.with_orientation(ShapeOrientation::YPositive)
                        .with_stroke_width(0.1)
                        .stroke_rect(rect);
                },
                self.point + parcel.coords * PARCEL_SIZE,
            )
            .color(palettes::css::LIME.with_alpha(0.9))
            .underlay(0.8)
            .transform(Transform::from_translation(Vec3::new(
                0.,
                cursor_height,
                0.,
            )))
            .insert_dyn(|layer| layer, layer)
    }
}

#[derive(Clone, PartialEq)]
pub struct FlatRectCursor {
    pub parcel: Entity,
    pub rect: IRect,
    pub height: f32,
}

impl ViewTemplate for FlatRectCursor {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        // Get the parcel component and realm component
        let parcel = cx.use_component::<Parcel>(self.parcel).unwrap();
        let realm = cx.use_component::<Realm>(parcel.realm);

        // We need the render layers from the realm.
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };

        Overlay::new()
            .shape_dyn(
                |r, sb| {
                    let rect = r.as_rect().inflate(0.1);
                    sb.with_orientation(ShapeOrientation::YPositive)
                        .with_stroke_width(0.1)
                        .stroke_rect(rect);
                },
                self.rect,
            )
            .color(palettes::css::LIME.with_alpha(0.9))
            .underlay(0.8)
            .transform(Transform::from_translation(Vec3::new(0., self.height, 0.)))
            .insert_dyn(|layer| layer, layer)
    }
}

#[derive(Clone, PartialEq)]
pub struct DecalRectCursor {
    pub parcel: Entity,
    pub rect: IRect,
}

impl ViewTemplate for DecalRectCursor {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        // Get the parcel component and realm component
        let parcel = cx.use_component::<Parcel>(self.parcel).unwrap();
        let realm = cx.use_component::<Realm>(parcel.realm);

        // We need the render layers from the realm.
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };

        // Look up the parcel's terrain contours. The terrain contour        // The bounds of the parcel in world space.
        let parcel_bounds = {
            let min = parcel.coords * PARCEL_SIZE;
            IRect::from_corners(min, min + IVec2::splat(PARCEL_SIZE - 1))
        };

        let mut rect = self.rect.intersect(parcel_bounds);
        rect.min -= parcel_bounds.min;
        rect.max -= parcel_bounds.min;

        // Note that assets is untracked because it's constantly changing.
        let shape_ref = parcel.center_shape();
        let ts_handle = cx.use_resource::<TerrainContoursHandle>().0.clone();
        let ts_assets = cx.use_resource_untracked::<Assets<TerrainContoursTableAsset>>();

        // Extract out the height map.
        let terrain_heights = ts_assets.get(&ts_handle).map(|contours| {
            let lock = contours.0.read().unwrap();
            let heights = lock.get(shape_ref.shape as usize).height.clone();
            heights
        });

        Overlay::new()
            .shape_dyn(
                |(rect, origin, heights, rotation), sb| {
                    sb.with_stroke_width(0.2)
                        .with_orientation(ShapeOrientation::YPositive);
                    if let Some(heights) = heights {
                        let rheights = rotator::RotatingSquareArray::new(
                            heights.size(),
                            rotation as i32,
                            heights.elts(),
                        );
                        let mut verts: Vec<Vec3> = Vec::with_capacity(PARCEL_SIZE_U * 4 + 3);
                        for x in rect.min.x..=rect.max.x {
                            verts.push(Vec3::new(
                                (origin.x + x) as f32,
                                rheights.get(x as usize, rect.min.y as usize) as f32 * 0.5 + 0.01,
                                (origin.y + rect.min.y) as f32,
                            ));
                        }
                        for z in (rect.min.y + 1)..(rect.max.y - 1) {
                            verts.push(Vec3::new(
                                (origin.x + rect.max.x) as f32,
                                rheights.get(rect.max.x as usize, z as usize) as f32 * 0.5 + 0.01,
                                (origin.y + z) as f32,
                            ));
                        }
                        for x in (rect.min.x..=rect.max.x).rev() {
                            verts.push(Vec3::new(
                                (origin.x + x) as f32,
                                rheights.get(x as usize, rect.max.y as usize) as f32 * 0.5 + 0.01,
                                (origin.y + rect.max.y) as f32,
                            ));
                        }
                        for z in ((rect.min.y + 1)..(rect.max.y - 1)).rev() {
                            verts.push(Vec3::new(
                                (origin.x + rect.min.x) as f32,
                                rheights.get(rect.min.x as usize, z as usize) as f32 * 0.5 + 0.01,
                                (origin.x + z) as f32,
                            ));
                        }
                        sb.stroke_polygon_3d(
                            &verts,
                            PolygonOptions {
                                closed: true,
                                ..default()
                            },
                        );
                    }
                },
                (rect, parcel_bounds.min, terrain_heights, shape_ref.rotation),
            )
            .color(palettes::css::LIME.with_alpha(0.9))
            .underlay(0.8)
            .insert_dyn(|layer| layer, layer)
    }
}
