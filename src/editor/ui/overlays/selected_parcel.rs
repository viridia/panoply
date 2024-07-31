use bevy::{
    asset::Assets,
    color::{palettes, Alpha},
    math::{Rect, Vec2, Vec3},
    prelude::Entity,
    render::view::RenderLayers,
};
use bevy_quill::{Cond, Cx, View, ViewTemplate};
use bevy_quill_overlays::{LinesBuilder, Overlay};

use crate::{
    editor::SelectedParcel,
    terrain::{
        rotator,
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel, PARCEL_SIZE_F, PARCEL_SIZE_U,
    },
    world::Realm,
};

#[derive(Clone, PartialEq)]
pub struct SelectedParcelOverlay;

impl ViewTemplate for SelectedParcelOverlay {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let parcel_id = cx.use_resource::<SelectedParcel>();

        Cond::new(
            parcel_id.0.is_some(),
            parcel_id.0.map(|parcel| SelectedParcelContour { parcel }),
            (),
        )
    }
}

#[derive(Clone, PartialEq)]
pub struct SelectedParcelContour {
    pub parcel: Entity,
}

impl ViewTemplate for SelectedParcelContour {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        let parcel = cx.use_component::<Parcel>(self.parcel).unwrap();
        let realm = cx.use_component::<Realm>(parcel.realm);
        let layer = match realm {
            Some(realm) => realm.layer.clone(),
            None => RenderLayers::none(),
        };
        let parcel_bounds = {
            let min = parcel.coords.as_vec2() * PARCEL_SIZE_F;
            Rect::from_corners(min, min + Vec2::splat(PARCEL_SIZE_F))
        };
        let shape_ref = parcel.center_shape();
        let ts_handle = cx.use_resource::<TerrainContoursHandle>().0.clone();
        let ts_assets = cx.use_resource_untracked::<Assets<TerrainContoursTableAsset>>();
        let terrain_heights = ts_assets.get(&ts_handle).map(|contours| {
            let lock = contours.0.lock().unwrap();
            let heights = lock.get(shape_ref.shape as usize).height.clone();
            heights
        });

        Overlay::new()
            .mesh_dyn(
                |(bounds, heights, rotation), sb: &mut LinesBuilder| {
                    if let Some(heights) = heights {
                        let rheights = rotator::RotatingSquareArray::new(
                            heights.size(),
                            rotation as i32,
                            heights.elts(),
                        );
                        for x in 0..=PARCEL_SIZE_U {
                            for z in 0..PARCEL_SIZE_U {
                                let a = Vec3::new(
                                    bounds.min.x + x as f32,
                                    rheights.get(x as i32, z as i32) as f32 * 0.5,
                                    bounds.min.y + z as f32,
                                );
                                let b = Vec3::new(
                                    bounds.min.x + x as f32,
                                    rheights.get(x as i32, z as i32 + 1) as f32 * 0.5,
                                    bounds.min.y + (z + 1) as f32,
                                );
                                sb.line(a, b);
                            }
                        }
                        for z in 0..=PARCEL_SIZE_U {
                            for x in 0..PARCEL_SIZE_U {
                                let a = Vec3::new(
                                    bounds.min.x + x as f32,
                                    rheights.get(x as i32, z as i32) as f32 * 0.5,
                                    bounds.min.y + z as f32,
                                );
                                let b = Vec3::new(
                                    bounds.min.x + (x + 1) as f32,
                                    rheights.get(x as i32 + 1, z as i32) as f32 * 0.5,
                                    bounds.min.y + z as f32,
                                );
                                sb.line(a, b);
                            }
                        }
                    }
                },
                (parcel_bounds, terrain_heights, shape_ref.rotation),
            )
            .color(palettes::css::YELLOW.with_alpha(0.8))
            .insert_dyn(|layer| layer, layer)
    }
}
