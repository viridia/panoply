//! Each editor mode's overlays, spawned as the mode is entered and despawned as it is left. Realm
//! mode's: a grid on the ground at parcel and precinct boundaries, and the realm's border. Terrain
//! mode's: the grid at parcel boundaries only, and the selected parcel's contour.
//!
//! The contour is drawn as lines (retained gizmos), not `panoply_overlays`' flat shapes: it follows
//! the ground's slopes, which a flat stroke seen edge-on would lose, and needs no underlay, which
//! shows where an overlay passes through scenery.

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    gizmos::{config::GizmoLineConfig, retained::Gizmo, GizmoAsset},
    prelude::*,
};
use panoply_core::{metrics::PRECINCT_SIZE, Realm, Viewpoint};
use panoply_overlays::{AddOverlay, DrawPlane, OverlayColor, OverlayShape, ShapeBuilder};
use panoply_terrain::{
    RotatingSquareArray, TerrainContoursHandle, TerrainContoursTableAsset, TerrainMap,
    TerrainMapAsset, PARCEL_HEIGHT_SCALE, PARCEL_SIZE, PARCEL_SIZE_F, PARCEL_SIZE_U,
};

use crate::{mode_selector::EditorMode, parcel_selection::ParcelSelection};

/// Points along each side of a parcel's contour: one a meter, both ends included.
const CONTOUR_STRIDE: usize = PARCEL_SIZE_U + 1;

/// How much nearer the camera the contour is drawn than it is: the contour is the ground before
/// smoothing, which rises over it in places. A fraction of the depth, not a distance (`Gizmo`).
const CONTOUR_DEPTH_BIAS: f32 = -0.01;

/// The grid's height, in meters.
const GRID_HEIGHT: f32 = 0.05;

/// The view realm's map, in parcels; empty while the view is in no realm.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct RealmGrid {
    parcels: IRect,
    /// Only the parcels' lines, as terrain mode draws it: no precincts, no border.
    parcels_only: bool,
}

impl OverlayShape for RealmGrid {
    fn draw(&self, builder: &mut ShapeBuilder) {
        // A realm with no parcels has no map to outline.
        if self.parcels.is_empty() {
            return;
        }
        let map = self.parcels.as_rect();
        let map = Rect::from_corners(map.min * PARCEL_SIZE as f32, map.max * PARCEL_SIZE as f32);
        // Just above flat ground at 0, where its depth would tie with the ground's.
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(GRID_HEIGHT);
        builder.with_stroke_width(0.1).grid(map, PARCEL_SIZE as f32);
        if self.parcels_only {
            return;
        }
        // Over the parcel lines, so a precinct's are brighter as well as wider.
        builder
            .with_stroke_width(0.3)
            .grid(map, PRECINCT_SIZE as f32);
        // Outside the map, so the border does not hide its outer parcels' edges.
        builder.with_stroke_width(0.5).stroke_rect(map.inflate(0.5));
    }
}

/// The selected parcel's contour: its heights at every meter, as the terrain tools edit them,
/// before the ground smooths them. Empty while no parcel is selected.
#[derive(Component, Clone, PartialEq, Default)]
pub struct SelectedContour {
    parcel: IVec2,
    /// `CONTOUR_STRIDE` rows of heights, one a meter along z, each one a meter along x.
    heights: Vec<f32>,
}

impl SelectedContour {
    /// The selected parcel and the height of each of its vertices, in its coordinates; `None`
    /// while no parcel is selected.
    pub(crate) fn heights(&self) -> Option<(IVec2, impl Fn(IVec2) -> f32 + '_)> {
        (!self.heights.is_empty()).then_some((self.parcel, |vertex: IVec2| {
            let vertex = vertex
                .clamp(IVec2::ZERO, IVec2::splat(PARCEL_SIZE))
                .as_uvec2();
            self.heights[vertex.y as usize * CONTOUR_STRIDE + vertex.x as usize]
        }))
    }

    /// The selected parcel, and how far along `ray` it first meets the contour from above, within
    /// `range`. The contour is drawn between its vertices, and runs on flat for
    /// [`CONTOUR_MARGIN`] past its edges, so that a vertex on an edge has as much ground that
    /// rounds to it as one inside.
    pub(crate) fn cast(&self, ray: Ray3d, range: f32) -> Option<(IVec2, f32)> {
        let (parcel, height) = self.heights()?;
        let origin = (parcel.as_vec2() * PARCEL_SIZE_F).extend(0.).xzy();
        let local = Ray3d::new(ray.origin - origin, ray.direction);
        Some((parcel, cast_contour(height, local, range)?))
    }
}

/// How far past its edges a contour is picked, in meters: half the distance between vertices.
const CONTOUR_MARGIN: f32 = 0.5;

/// How far a pick's ray steps over a contour between tests, in meters.
const CAST_STEP: f32 = 0.05;

/// How far along `ray`, in a parcel's coordinates, it first meets the surface between the
/// vertices of `height`, from above, within `range`.
fn cast_contour(height: impl Fn(IVec2) -> f32, ray: Ray3d, range: f32) -> Option<f32> {
    let surface = |p: Vec2| {
        let p = p.clamp(Vec2::ZERO, Vec2::splat(PARCEL_SIZE_F));
        let corner = p.floor().min(Vec2::splat(PARCEL_SIZE_F - 1.));
        let f = p - corner;
        let c = corner.as_ivec2();
        let near = height(c).lerp(height(c + IVec2::X), f.x);
        let far = height(c + IVec2::Y).lerp(height(c + IVec2::ONE), f.x);
        near.lerp(far, f.y)
    };
    let (low, high) = (0..CONTOUR_STRIDE as i32)
        .flat_map(|z| (0..CONTOUR_STRIDE as i32).map(move |x| IVec2::new(x, z)))
        .map(&height)
        .fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(h), hi.max(h)));
    // Where the ray is within the box that holds the contour and its margin, a little taller than
    // the contour, so that a ray entering it through its top starts above the surface.
    let min = Vec3::new(-CONTOUR_MARGIN, low - 0.01, -CONTOUR_MARGIN);
    let max = Vec3::new(
        PARCEL_SIZE_F + CONTOUR_MARGIN,
        high + 0.01,
        PARCEL_SIZE_F + CONTOUR_MARGIN,
    );
    let (mut start, mut end) = (0f32, range);
    for axis in 0..3 {
        let (o, d) = (ray.origin[axis], ray.direction[axis]);
        if d.abs() < 1e-6 {
            if o < min[axis] || o > max[axis] {
                return None;
            }
            continue;
        }
        let (a, b) = ((min[axis] - o) / d, (max[axis] - o) / d);
        start = start.max(a.min(b));
        end = end.min(a.max(b));
    }
    if start > end {
        return None;
    }
    let above = |t: f32| {
        let p = ray.get_point(t);
        p.y - surface(p.xz())
    };
    // A ray that enters the box below the surface passes under the contour's edge.
    if above(start) < 0. {
        return None;
    }
    let (mut t0, mut t) = (start, start);
    while t < end {
        t = (t + CAST_STEP).min(end);
        if above(t) <= 0. {
            for _ in 0..16 {
                let mid = (t0 + t) / 2.;
                if above(mid) > 0. {
                    t0 = mid;
                } else {
                    t = mid;
                }
            }
            return Some(t);
        }
        t0 = t;
    }
    None
}

/// The heavier line around the selected contour; a child of the [`SelectedContour`], drawn with it.
#[derive(Component)]
struct SelectedContourBorder;

pub struct ModeOverlaysPlugin;

impl Plugin for ModeOverlaysPlugin {
    fn build(&self, app: &mut App) {
        app.add_overlay::<RealmGrid>()
            .add_systems(
                OnEnter(EditorMode::Realm),
                spawn_realm_grid(EditorMode::Realm),
            )
            .add_systems(
                OnEnter(EditorMode::Terrain),
                spawn_realm_grid(EditorMode::Terrain),
            )
            .add_systems(
                Update,
                update_realm_grid.run_if(any_with_component::<RealmGrid>),
            )
            .add_systems(OnEnter(EditorMode::Terrain), spawn_selected_contour)
            .add_systems(
                Update,
                (update_selected_contour, draw_selected_contour)
                    .chain()
                    .in_set(SelectedContourSystems)
                    .run_if(in_state(EditorMode::Terrain)),
            );
    }
}

/// Updates the [`SelectedContour`], which the terrain tools read.
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct SelectedContourSystems;

/// Spawns `mode`'s grid: in terrain mode, only the parcels' lines.
fn spawn_realm_grid(mode: EditorMode) -> impl Fn(Commands) {
    move |mut commands: Commands| {
        commands.spawn((
            Name::new("RealmGrid"),
            RealmGrid {
                parcels_only: mode == EditorMode::Terrain,
                ..default()
            },
            OverlayColor {
                base: Color::WHITE.with_alpha(0.15),
                underlay: 0.3,
            },
            Propagate(RenderLayers::none()),
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(mode),
        ));
    }
}

/// Follows the view's realm: its map and its render layer.
fn update_realm_grid(
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    mut grids: Query<(&mut RealmGrid, &mut Propagate<RenderLayers>)>,
) {
    let realm = viewpoint.realm.and_then(|e| realms.get(e).ok());
    let parcels = realm.map_or(IRect::default(), |r| r.parcel_bounds);
    let layer = realm.map_or(RenderLayers::none(), |r| r.layer.clone());
    for (mut grid, mut propagate) in &mut grids {
        let parcels_only = grid.parcels_only;
        grid.set_if_neq(RealmGrid {
            parcels,
            parcels_only,
        });
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}

fn spawn_selected_contour(mut gizmos: ResMut<Assets<GizmoAsset>>, mut commands: Commands) {
    let gizmo = |gizmos: &mut Assets<GizmoAsset>, width| Gizmo {
        handle: gizmos.add(GizmoAsset::default()),
        line_config: GizmoLineConfig { width, ..default() },
        depth_bias: CONTOUR_DEPTH_BIAS,
    };
    commands.spawn((
        Name::new("SelectedContour"),
        SelectedContour::default(),
        gizmo(&mut gizmos, 1.0),
        Propagate(RenderLayers::none()),
        DespawnOnExit(EditorMode::Terrain),
        children![(
            Name::new("SelectedContourBorder"),
            SelectedContourBorder,
            gizmo(&mut gizmos, 3.0),
        )],
    ));
}

/// Follows the selection, the view's realm and the contour: the selected parcel's shape and turn
/// from the realm's map, and its heights from the contours table.
fn update_selected_contour(
    selection: Res<ParcelSelection>,
    viewpoint: Res<Viewpoint>,
    realms: Query<(&Realm, &TerrainMap)>,
    maps: Res<Assets<TerrainMapAsset>>,
    contours: Res<TerrainContoursHandle>,
    tables: Res<Assets<TerrainContoursTableAsset>>,
    mut overlays: Query<(&mut SelectedContour, &mut Propagate<RenderLayers>)>,
) {
    let realm = viewpoint.realm.and_then(|e| realms.get(e).ok());
    let layer = realm.map_or(RenderLayers::none(), |(realm, _)| realm.layer.clone());
    let contour = realm
        .and_then(|(realm, terrain)| {
            let parcel = selection.in_realm(realm)?;
            let shape = maps.get(&terrain.handle)?.shape_at(parcel);
            let table = tables.get(&contours.0)?.0.read().unwrap();
            let height = &table.get(shape.shape as usize).height;
            let turned = RotatingSquareArray::new(height.size(), shape.rotation, height.elts());
            let heights = (0..CONTOUR_STRIDE)
                .flat_map(|z| (0..CONTOUR_STRIDE).map(move |x| (x, z)))
                .map(|(x, z)| turned.get(x, z) as f32 * PARCEL_HEIGHT_SCALE)
                .collect();
            Some(SelectedContour { parcel, heights })
        })
        .unwrap_or_default();
    for (mut overlay, mut propagate) in &mut overlays {
        overlay.set_if_neq(contour.clone());
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}

/// Redraws the contour when it changes: a line along every meter each way, and the border.
fn draw_selected_contour(
    mut overlays: Query<(&SelectedContour, &Gizmo, &mut Transform), Changed<SelectedContour>>,
    borders: Query<(&Gizmo, &ChildOf), With<SelectedContourBorder>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) {
    for (border, child_of) in &borders {
        let Ok((contour, lattice, mut transform)) = overlays.get_mut(child_of.parent()) else {
            continue;
        };
        transform.translation = (contour.parcel.as_vec2() * PARCEL_SIZE_F).extend(0.).xzy();
        let mut lines = GizmoAsset::default();
        let mut outline = GizmoAsset::default();
        if !contour.heights.is_empty() {
            let point = |x: usize, z: usize| {
                Vec3::new(x as f32, contour.heights[z * CONTOUR_STRIDE + x], z as f32)
            };
            let color = css::YELLOW.with_alpha(0.8);
            for i in 0..CONTOUR_STRIDE {
                lines.linestrip((0..CONTOUR_STRIDE).map(|j| point(i, j)), color);
                lines.linestrip((0..CONTOUR_STRIDE).map(|j| point(j, i)), color);
            }
            // Around the parcel, back to the start.
            let n = PARCEL_SIZE_U;
            let around = (0..n)
                .map(|x| point(x, 0))
                .chain((0..n).map(|z| point(n, z)))
                .chain((0..n).map(|x| point(n - x, n)))
                .chain((0..=n).map(|z| point(0, n - z)));
            outline.linestrip(around, css::YELLOW);
        }
        for (gizmo, asset) in [(lattice, lines), (border, outline)] {
            if let Some(mut current) = gizmos.get_mut(&gizmo.handle) {
                *current = asset;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down_at(x: f32, z: f32) -> Ray3d {
        Ray3d::new(Vec3::new(x, 10., z), Dir3::NEG_Y)
    }

    #[test]
    fn a_ray_down_meets_a_slope_between_its_vertices() {
        // Rising a meter a meter along x.
        let slope = |v: IVec2| v.x as f32;
        let t = cast_contour(slope, down_at(2.25, 7.), 100.).unwrap();
        assert!((t - 7.75).abs() < 1e-3, "{t}");
    }

    #[test]
    fn a_contour_is_picked_half_a_meter_past_its_edges() {
        let flat = |_| 1.;
        let past = cast_contour(flat, down_at(-0.4, PARCEL_SIZE_F + 0.4), 100.).unwrap();
        assert!((past - 9.).abs() < 1e-3, "{past}");
        assert_eq!(cast_contour(flat, down_at(-0.6, 4.), 100.), None);
        assert_eq!(cast_contour(flat, down_at(4., 4.), 5.), None);
    }

    #[test]
    fn a_ray_under_the_edge_misses() {
        // A ray rising toward the parcel from beside it, below its edge at height 2, and rising
        // more slowly than the parcel's slope.
        let slope = |v: IVec2| 2. + v.x as f32 * 0.25;
        let ray = Ray3d::new(
            Vec3::new(-3., 1.5, 4.),
            Dir3::new(Vec3::new(1., 0.05, 0.)).unwrap(),
        );
        assert_eq!(cast_contour(slope, ray, 100.), None);
    }
}
