//! The sculpting tools' cursor in terrain mode: over the selected parcel, a square around the
//! vertex a brush would change, or, during a rectangle tool's drag, the rectangle from the vertex
//! where the drag started to the one under the pointer. It is flat, at the vertex's height, or a
//! rectangle's at its start's: the height a flatten would leave.
//!
//! Hovering, the pointer is where the ground was hit, snapped to the nearest vertex; off the
//! parcel, no cursor shows. A drag is the primary button held since a press on the selected
//! parcel. The vertex is clamped to the parcel, and the tool reads the pointer's ray itself: a
//! brush takes the ground's hit, or where there is none a level plane at the last height; a
//! rectangle a level plane at its start's height, so that its far corner stays under the pointer.
//! Over the selected parcel, the editor's picking hits the contour as drawn (`parcel_selection`).

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    picking::{
        backend::ray::RayMap,
        events::PointerState,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
};
use panoply_core::{PrimaryCamera, Realm, Viewpoint};
use panoply_overlays::{AddOverlay, DrawPlane, OverlayColor, OverlayShape, ShapeBuilder};
use panoply_terrain::{Parcel, PARCEL_SIZE};

use crate::{
    mode_overlays::{SelectedContour, SelectedContourSystems},
    mode_selector::{EditorMode, TerrainTool},
};

/// How far the cursor reaches past its vertices, in meters.
const CURSOR_MARGIN: f32 = 0.2;

/// The cursor's stroke, in meters.
const CURSOR_STROKE: f32 = 0.08;

/// How far above its vertices the cursor is drawn, so its depth does not tie with flat ground's.
const CURSOR_LIFT: f32 = 0.02;

/// A parcel's vertices, in its coordinates.
const PARCEL_VERTICES: IRect = IRect {
    min: IVec2::ZERO,
    max: IVec2::splat(PARCEL_SIZE),
};

/// The cursor: the vertices it surrounds, in the world's x and z, and its height. Hidden while
/// `None`.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct TerrainCursor(Option<(IRect, f32)>);

impl OverlayShape for TerrainCursor {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let Some((vertices, height)) = self.0 else {
            return;
        };
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(height + CURSOR_LIFT)
            .with_stroke_width(CURSOR_STROKE)
            .stroke_rect(vertices.as_rect().inflate(CURSOR_MARGIN));
    }
}

/// A drag in progress, in the selected parcel's coordinates.
#[derive(Clone, Copy, Debug)]
struct TerrainDrag {
    /// The vertex where it started.
    anchor: IVec2,
    /// The vertex under the pointer, kept while the pointer has no ray.
    vertex: IVec2,
    /// The height of the level plane the pointer's ray is met on where it misses the ground.
    plane: f32,
}

pub struct TerrainCursorPlugin;

impl Plugin for TerrainCursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_overlay::<TerrainCursor>()
            .add_systems(OnEnter(EditorMode::Terrain), spawn_terrain_cursor)
            .add_systems(
                Update,
                update_terrain_cursor
                    .after(SelectedContourSystems)
                    .run_if(in_state(EditorMode::Terrain)),
            );
    }
}

fn spawn_terrain_cursor(mut commands: Commands) {
    commands.spawn((
        Name::new("TerrainCursor"),
        TerrainCursor::default(),
        cursor_color(false),
        Propagate(RenderLayers::none()),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(EditorMode::Terrain),
    ));
}

/// The cursor's color, brighter during a drag.
fn cursor_color(dragging: bool) -> OverlayColor {
    let base = if dragging {
        css::LIGHT_CYAN
    } else {
        css::AQUAMARINE
    };
    OverlayColor {
        base: base.with_alpha(0.9).into(),
        underlay: 0.8,
    }
}

/// The pointer's place on the ground, while it has one.
#[derive(Clone, Copy, Default)]
struct PointerOnGround {
    /// Where the ground was hit, by the hovered parcel's picking.
    hit: Option<Vec3>,
    /// The pointer's ray through the view.
    ray: Option<Ray3d>,
    /// Where the press that holds the primary button hit the selected parcel.
    press: Option<Vec3>,
}

/// Follows the pointer, the tool and the selected contour; starts and ends drags.
fn update_terrain_cursor(
    tool: Res<State<TerrainTool>>,
    viewpoint: Res<Viewpoint>,
    contours: Query<&SelectedContour>,
    parcels: Query<&Parcel>,
    realms: Query<&Realm>,
    pointers: Res<PointerState>,
    hovers: Res<HoverMap>,
    rays: Res<RayMap>,
    cameras: Query<(), With<PrimaryCamera>>,
    mut drag: Local<Option<TerrainDrag>>,
    mut cursors: Query<(
        &mut TerrainCursor,
        &mut OverlayColor,
        &mut Propagate<RenderLayers>,
    )>,
) {
    let realm = viewpoint.realm;
    let selected = contours.iter().find_map(SelectedContour::heights);
    let pointer = selected
        .as_ref()
        .zip(realm)
        .map(|((parcel, _), realm)| PointerOnGround {
            hit: hovers.get(&PointerId::Mouse).and_then(|hits| {
                hits.iter()
                    .find_map(|(&e, hit)| parcels.contains(e).then_some(hit.position)?)
            }),
            ray: rays
                .iter()
                .find(|(id, _)| id.pointer == PointerId::Mouse && cameras.contains(id.camera))
                .map(|(_, &ray)| ray),
            press: pointers
                .get(PointerId::Mouse, PointerButton::Primary)
                .and_then(|state| {
                    state.pressing.iter().find_map(|(&e, (_, _, hit))| {
                        let p = parcels.get(e).ok()?;
                        (p.realm == realm && p.coords == *parcel).then_some(hit.position)?
                    })
                }),
        })
        .unwrap_or_default();
    if pointer.press.is_none() {
        *drag = None;
    }

    let shape = match *tool.get() {
        TerrainTool::RaiseDraw | TerrainTool::LowerDraw | TerrainTool::FlattenDraw => {
            Some(CursorShape::Brush)
        }
        TerrainTool::RaiseRect | TerrainTool::LowerRect | TerrainTool::FlattenRect => {
            Some(CursorShape::Rect)
        }
        // TODO(PAN38e): the flora tools' cell cursor.
        _ => None,
    };
    let cursor = selected.zip(shape).and_then(|((parcel, height), shape)| {
        let origin = parcel * PARCEL_SIZE;
        let local = |point: Vec3| nearest_vertex(point.xz() - origin.as_vec2());
        let (vertices, height) = match (pointer.press, drag.as_mut()) {
            (None, _) => {
                let vertex = local(pointer.hit?);
                PARCEL_VERTICES.contains(vertex).then(|| {
                    (IRect::from_corners(vertex, vertex), height(vertex))
                })?
            }
            (Some(press), None) => {
                let anchor = local(press).clamp(PARCEL_VERTICES.min, PARCEL_VERTICES.max);
                let start = TerrainDrag {
                    anchor,
                    vertex: anchor,
                    plane: height(anchor),
                };
                *drag = Some(start);
                (IRect::from_corners(anchor, anchor), start.plane)
            }
            (Some(_), Some(drag)) => {
                let hit = pointer.hit.filter(|_| shape == CursorShape::Brush);
                if let Some(hit) = hit {
                    drag.plane = hit.y;
                }
                let point = hit.or_else(|| level_point(pointer.ray?, drag.plane));
                if let Some(point) = point {
                    drag.vertex = local(point).clamp(PARCEL_VERTICES.min, PARCEL_VERTICES.max);
                }
                match shape {
                    CursorShape::Brush => {
                        (IRect::from_corners(drag.vertex, drag.vertex), height(drag.vertex))
                    }
                    CursorShape::Rect => (
                        IRect::from_corners(drag.anchor, drag.vertex),
                        height(drag.anchor),
                    ),
                }
            }
        };
        let world = IRect::from_corners(vertices.min + origin, vertices.max + origin);
        Some((world, height))
    });

    let layer = realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    for (mut overlay, mut color, mut propagate) in &mut cursors {
        overlay.set_if_neq(TerrainCursor(cursor));
        color.set_if_neq(cursor_color(drag.is_some()));
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}

/// What a tool's cursor surrounds.
#[derive(Clone, Copy, PartialEq, Debug)]
enum CursorShape {
    /// The vertex under the pointer.
    Brush,
    /// The vertices from the drag's start to the one under the pointer.
    Rect,
}

/// The vertex nearest `point`, a point in a parcel's coordinates, in meters.
fn nearest_vertex(point: Vec2) -> IVec2 {
    point.round().as_ivec2()
}

/// Where `ray` meets the level plane at `height`, if it does ahead of its origin.
fn level_point(ray: Ray3d, height: f32) -> Option<Vec3> {
    let distance = ray.intersect_plane(Vec3::Y * height, InfinitePlane3d::new(Vec3::Y))?;
    Some(ray.get_point(distance))
}

#[cfg(test)]
mod tests {
    use panoply_terrain::PARCEL_SIZE_F;

    use super::*;

    #[test]
    fn a_point_snaps_to_the_vertex_nearest_it() {
        assert_eq!(nearest_vertex(Vec2::new(3.4, 7.6)), IVec2::new(3, 8));
        assert_eq!(nearest_vertex(Vec2::new(-0.4, 16.4)), IVec2::new(0, 16));
        assert!(!PARCEL_VERTICES.contains(nearest_vertex(Vec2::new(-0.6, 4.))));
        assert!(PARCEL_VERTICES.contains(nearest_vertex(Vec2::new(PARCEL_SIZE_F + 0.4, 4.))));
    }

    #[test]
    fn a_ray_down_meets_the_level_plane_below_it() {
        let ray = Ray3d::new(Vec3::new(0., 10., 0.), Dir3::new(Vec3::new(0.6, -0.8, 0.)).unwrap());
        let point = level_point(ray, 2.).unwrap();
        assert!(point.distance(Vec3::new(6., 2., 0.)) < 1e-4, "{point}");
        assert_eq!(level_point(ray, 12.), None);
    }
}
