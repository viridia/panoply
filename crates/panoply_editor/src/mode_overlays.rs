//! Each editor mode's overlays, spawned as the mode is entered and despawned as it is left. Realm
//! mode's: a grid on the ground at parcel and precinct boundaries, and the realm's border. Terrain
//! mode's: the selected parcel's contour.
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
#[derive(Component, Clone, PartialEq, Default)]
pub struct RealmGrid {
    parcels: IRect,
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
        builder.with_draw_plane(DrawPlane::XZ).with_offset(GRID_HEIGHT);
        builder.with_stroke_width(0.1).grid(map, PARCEL_SIZE as f32);
        // Over the parcel lines, so a precinct's are brighter as well as wider.
        builder.with_stroke_width(0.3).grid(map, PRECINCT_SIZE as f32);
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

/// The heavier line around the selected contour; a child of the [`SelectedContour`], drawn with it.
#[derive(Component)]
struct SelectedContourBorder;

pub struct ModeOverlaysPlugin;

impl Plugin for ModeOverlaysPlugin {
    fn build(&self, app: &mut App) {
        app.add_overlay::<RealmGrid>()
            .add_systems(OnEnter(EditorMode::Realm), spawn_realm_grid)
            .add_systems(Update, update_realm_grid.run_if(in_state(EditorMode::Realm)))
            .add_systems(OnEnter(EditorMode::Terrain), spawn_selected_contour)
            .add_systems(
                Update,
                (update_selected_contour, draw_selected_contour)
                    .chain()
                    .run_if(in_state(EditorMode::Terrain)),
            );
    }
}

fn spawn_realm_grid(mut commands: Commands) {
    commands.spawn((
        Name::new("RealmGrid"),
        RealmGrid::default(),
        OverlayColor {
            base: Color::WHITE.with_alpha(0.15),
            underlay: 0.3,
        },
        Propagate(RenderLayers::none()),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(EditorMode::Realm),
    ));
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
        grid.set_if_neq(RealmGrid { parcels });
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
