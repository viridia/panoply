use bevy::{
    asset::Assets,
    color::{palettes, Alpha},
    math::{Rect, Vec2},
    prelude::*,
    render::view::RenderLayers,
};
use bevy_quill::{Cx, Dynamic, IntoViewChild, View, ViewTemplate};
use bevy_quill_overlays::{Overlay, ShapeOrientation};

use crate::{
    editor::{ParcelCursor, SelectedParcel},
    terrain::{
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel, PARCEL_SIZE, PARCEL_SIZE_U,
    },
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct TerrainCursorOverlay;

impl ViewTemplate for TerrainCursorOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        // let tool = *cx.use_resource::<State<TerrainTool>>().get();
        let cursor = match cx.use_resource::<SelectedParcel>().0 {
            Some(_) => cx.use_resource::<ParcelCursor>().clone(),
            None => ParcelCursor::None,
        };

        Dynamic::new(match cursor {
            ParcelCursor::None => ().into_view_child(),
            ParcelCursor::Point((parcel, pos)) => {
                PointCursor { parcel, point: pos }.into_view_child()
            }
            ParcelCursor::FlatRect((parcel, rect, height)) => FlatRectCursor {
                parcel,
                rect,
                height,
            }
            .into_view_child(),
            ParcelCursor::DecalRect((_, _rect)) => ().into_view_child(),
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
                let lock = contours.0.lock().unwrap();
                let pos = self.point - parcel.coords * PARCEL_SIZE;
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
                self.point,
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
    pub point: IVec2,
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

        // Look up the parcel's terrain contours. The terrain contour table is an asset.
        // Note that assets is untracked because it's constantly changing.
        let shape_ref = parcel.center_shape();
        let ts_handle = cx.use_resource::<TerrainContoursHandle>().0.clone();
        let ts_assets = cx.use_resource_untracked::<Assets<TerrainContoursTableAsset>>();

        // Extract out the height map.
        let cursor_height = ts_assets
            .get(&ts_handle)
            .map(|contours| {
                let lock = contours.0.lock().unwrap();
                let pos = self.point - parcel.coords * PARCEL_SIZE;
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
                self.point,
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
