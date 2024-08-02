use bevy::{
    asset::Assets,
    color::{palettes, Alpha},
    math::{Rect, Vec2},
    prelude::*,
    render::view::RenderLayers,
};
use bevy_quill::{Cond, Cx, View, ViewTemplate};
use bevy_quill_overlays::{LinesBuilder, Overlay, ShapeOrientation};

use crate::{
    editor::{ParcelCursor, SelectedParcel, TerrainTool},
    terrain::{
        rotator,
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel,
    },
    view::Viewpoint,
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct TerrainCursorOverlay;

impl ViewTemplate for TerrainCursorOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let tool = *cx.use_resource::<State<TerrainTool>>().get();
        let cursor = cx.use_resource::<ParcelCursor>().clone();
        let viewpoint = cx.use_resource::<Viewpoint>();
        let realm = viewpoint
            .realm
            .map(|r| cx.use_component::<Realm>(r).unwrap());
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };

        Overlay::new()
            .shape_dyn(
                |(cursor, tool), sb| {
                    match cursor {
                        ParcelCursor::None => {}
                        ParcelCursor::Point((_, pos, _alt)) => {
                            let rect = Rect::from_center_size(pos.as_vec2(), Vec2::splat(0.35));
                            sb.with_orientation(ShapeOrientation::YPositive)
                                .with_stroke_width(0.1)
                                .stroke_rect(rect);
                        }
                        ParcelCursor::Rect((_, rect)) => {
                            let vrect = Rect::from_corners(rect.min.as_vec2(), rect.max.as_vec2());
                            sb.with_orientation(ShapeOrientation::YPositive)
                                .with_stroke_width(0.1)
                                .stroke_rect(vrect);
                        }
                    };
                },
                (cursor, tool),
            )
            .color(palettes::css::LIME.with_alpha(0.5))
            .insert_dyn(|layer| layer, layer)
    }
}
