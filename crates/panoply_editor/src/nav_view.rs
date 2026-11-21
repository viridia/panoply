//! The navigation view, the Show menu's Navigation: each precinct's [`PrecinctNavMesh`], its
//! polygons filled by area and its edges coloured by what lies across them, so doorways, walls and
//! the seams with the neighbouring precincts stand out; and each actor's [`PlannedRoute`], a line
//! ending in an arrowhead. Rebuilt only when a mesh or route changes; the Show menu only hides and
//! shows it.
//!
//! After `bevy_rerecast`'s `debug.rs` (<https://github.com/janhohenheim/rerecast>, MIT or
//! Apache-2.0): outlines into a retained gizmo, and a translucent fill. Its fill is an unlit
//! `StandardMaterial`; this one is a `panoply_overlays` overlay, which still shows, fainter, where
//! the ground or scenery hides it.

use bevy::{
    app::Propagate,
    color::palettes::tailwind,
    gizmos::{config::GizmoLineConfig, retained::Gizmo, GizmoAsset},
    prelude::*,
};
use panoply_core::{Realm, Viewpoint};
use panoply_nav::{NavAreaKind, NavEdge, NavMesh, PlannedRoute, PrecinctNavMesh};
use panoply_overlays::{
    AddOverlay, OverlayColor, OverlayShape, PolygonOptions, ShapeBuilder, StrokeMarker,
};
use panoply_scenery::precinct::Precinct;

use crate::debug_views::DebugViews;

/// How far above its polygons the fill is drawn, so its depth does not tie with the ground's.
const FILL_LIFT: f32 = 0.02;

/// How much nearer the camera the edges are drawn than they are, as the physics view's lines are.
/// A fraction of the depth, not a distance (`Gizmo`).
const EDGE_DEPTH_BIAS: f32 = -0.001;

/// How opaque the fills and edges are where the ground or scenery hides them, as a fraction of
/// their alpha: enough to follow a polygon that runs under the terrain.
const UNDERLAY: f32 = 0.8;
const EDGE_UNDERLAY: f32 = 0.35;

/// How far above the ground a route is drawn, as faery drew them, and how wide.
const ROUTE_LIFT: f32 = 0.06;
const ROUTE_WIDTH: f32 = 0.08;

/// The view of one actor's route, holding its [`NavRouteLine`].
#[derive(Component)]
pub struct NavRouteView {
    actor: Entity,
}

/// A route's line, through its waypoints, ending in an arrowhead.
#[derive(Component, Clone, PartialEq, Default)]
struct NavRouteLine(Vec<Vec3>);

impl OverlayShape for NavRouteLine {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let options = PolygonOptions {
            end_marker: StrokeMarker::Arrowhead,
            ..default()
        };
        builder
            .with_stroke_width(ROUTE_WIDTH)
            .with_offset(ROUTE_LIFT)
            .stroke_polyline_3d(&self.0, options);
    }
}

/// The view of one precinct's mesh, a child of the precinct: its edges, and a [`NavAreaFill`]
/// child for each area kind the mesh has.
#[derive(Component)]
pub struct NavMeshView;

/// The polygons of one area kind, filled, each as its points in order around it.
#[derive(Component, Clone, PartialEq, Default)]
struct NavAreaFill(Vec<Vec<Vec3>>);

impl OverlayShape for NavAreaFill {
    fn draw(&self, builder: &mut ShapeBuilder) {
        for polygon in &self.0 {
            builder.fill_convex_3d(polygon);
        }
    }
}

pub struct NavViewPlugin;

impl Plugin for NavViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_overlay::<NavAreaFill>()
            .add_overlay::<NavRouteLine>()
            .add_observer(remove_view)
            .add_observer(remove_route_view)
            .add_systems(
                Update,
                (
                    rebuild_views,
                    rebuild_route_views,
                    show_views.run_if(resource_changed::<DebugViews>),
                ),
            );
    }
}

fn visibility(views: &DebugViews) -> Visibility {
    if views.navigation {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

fn area_color(area: NavAreaKind) -> Color {
    match area {
        NavAreaKind::Walkable => tailwind::SKY_500.with_alpha(0.3).into(),
        NavAreaKind::Door => tailwind::AMBER_400.with_alpha(0.6).into(),
        NavAreaKind::Path => tailwind::VIOLET_400.with_alpha(0.4).into(),
        NavAreaKind::Portal => tailwind::EMERALD_400.with_alpha(0.6).into(),
    }
}

fn edge_color(edge: NavEdge) -> Color {
    match edge {
        NavEdge::Wall => tailwind::ROSE_500.into(),
        NavEdge::Neighbor(_) => tailwind::SKY_200.with_alpha(0.6).into(),
        NavEdge::Boundary(_) => tailwind::LIME_400.into(),
    }
}

/// Replaces the view of each precinct whose mesh is new or changed.
fn rebuild_views(
    precincts: Query<
        (Entity, &PrecinctNavMesh, &Precinct, Option<&Children>),
        Changed<PrecinctNavMesh>,
    >,
    old_views: Query<(), With<NavMeshView>>,
    views: Res<DebugViews>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    mut commands: Commands,
) {
    for (entity, PrecinctNavMesh(mesh), precinct, children) in &precincts {
        for &child in children.into_iter().flatten() {
            if old_views.contains(child) {
                commands.entity(child).despawn();
            }
        }
        // Drawn anyway, as far as it can be, since the view is how a bad mesh is looked into.
        if let Err(error) = mesh.check() {
            warn!("Precinct {}: bad navigation mesh: {error}", precinct.coords);
        }
        let mut lines = GizmoAsset::default();
        let mut hidden_lines = GizmoAsset::default();
        for (a, b, edge) in edges(mesh) {
            let color = edge_color(edge);
            lines.line(a, b, color);
            hidden_lines.line(a, b, color.with_alpha(color.alpha() * EDGE_UNDERLAY));
        }
        let line_config = GizmoLineConfig {
            width: 2.0,
            ..default()
        };
        let view = commands
            .spawn((
                Name::new("NavMeshView"),
                NavMeshView,
                Gizmo {
                    handle: gizmos.add(lines),
                    line_config: line_config.clone(),
                    depth_bias: EDGE_DEPTH_BIAS,
                },
                Propagate(precinct.render_layer.clone()),
                Transform::default(),
                visibility(&views),
                ChildOf(entity),
            ))
            .id();
        // Retained gizmos have no underlay, so the edges are drawn again, faint, in front of
        // everything (a depth bias of -1), as the fills' underlay shows them where hidden.
        commands.spawn((
            Name::new("NavEdgeUnderlay"),
            Gizmo {
                handle: gizmos.add(hidden_lines),
                line_config,
                depth_bias: -1.,
            },
            Transform::default(),
            ChildOf(view),
        ));
        for (area, polygons) in fills(mesh) {
            commands.spawn((
                Name::new("NavAreaFill"),
                NavAreaFill(polygons),
                OverlayColor {
                    base: area_color(area),
                    underlay: UNDERLAY,
                },
                Transform::default(),
                ChildOf(view),
            ));
        }
    }
}

/// Despawns a precinct's view when its mesh is removed.
fn remove_view(
    remove: On<Remove<PrecinctNavMesh>>,
    children: Query<&Children>,
    views: Query<(), With<NavMeshView>>,
    mut commands: Commands,
) {
    for &child in children.get(remove.entity).into_iter().flatten() {
        if views.contains(child) {
            // The precinct may be despawning, and its children with it.
            commands.entity(child).try_despawn();
        }
    }
}

/// Draws each new or changed route, in the viewed realm's layer.
fn rebuild_route_views(
    routes: Query<(Entity, &PlannedRoute), Changed<PlannedRoute>>,
    route_views: Query<(&NavRouteView, &Children)>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    views: Res<DebugViews>,
    mut commands: Commands,
) {
    for (actor, PlannedRoute(route)) in &routes {
        let line = NavRouteLine(route.waypoints.clone());
        if let Some((_, children)) = route_views.iter().find(|(view, _)| view.actor == actor) {
            for &child in children {
                commands.entity(child).insert(line.clone());
            }
            continue;
        }
        let Some(realm) = viewpoint.realm.and_then(|realm| realms.get(realm).ok()) else {
            continue;
        };
        commands.spawn((
            Name::new("NavRouteView"),
            NavRouteView { actor },
            Propagate(realm.layer.clone()),
            Transform::default(),
            visibility(&views),
            children![(
                Name::new("NavRouteLine"),
                line,
                OverlayColor {
                    base: tailwind::YELLOW_300.with_alpha(0.8).into(),
                    underlay: UNDERLAY,
                },
                Transform::default(),
            )],
        ));
    }
}

/// Despawns an actor's route view when its route is removed, or the actor despawns.
fn remove_route_view(
    remove: On<Remove<PlannedRoute>>,
    route_views: Query<(Entity, &NavRouteView)>,
    mut commands: Commands,
) {
    for (view, NavRouteView { actor }) in &route_views {
        if *actor == remove.entity {
            commands.entity(view).try_despawn();
        }
    }
}

fn show_views(
    views: Res<DebugViews>,
    mut q_views: Query<&mut Visibility, Or<(With<NavMeshView>, With<NavRouteView>)>>,
) {
    for mut shown in &mut q_views {
        shown.set_if_neq(visibility(&views));
    }
}

/// Each edge once, with what lies across it: an edge two polygons share is taken from the one
/// with the lower index. Edges naming a vertex the mesh lacks are left out.
fn edges(mesh: &NavMesh) -> impl Iterator<Item = (Vec3, Vec3, NavEdge)> + '_ {
    let point = |v: u32| mesh.vertices.get(v as usize).copied();
    mesh.polygons
        .iter()
        .enumerate()
        .flat_map(move |(index, polygon)| {
            let count = polygon.vertices.len();
            polygon
                .edges
                .iter()
                .enumerate()
                .filter(move |(_, edge)| !matches!(edge, NavEdge::Neighbor(other) if (*other as usize) < index))
                .filter_map(move |(i, &edge)| {
                    let a = point(*polygon.vertices.get(i)?)?;
                    let b = point(polygon.vertices[(i + 1) % count])?;
                    Some((a, b, edge))
                })
        })
}

/// The polygons grouped by area, each lifted by [`FILL_LIFT`], in the order their areas first
/// appear. Polygons naming a vertex the mesh lacks are left out.
fn fills(mesh: &NavMesh) -> Vec<(NavAreaKind, Vec<Vec<Vec3>>)> {
    let lift = |p: Vec3| p + Vec3::Y * FILL_LIFT;
    let mut areas: Vec<(NavAreaKind, Vec<Vec<Vec3>>)> = Vec::new();
    for polygon in &mesh.polygons {
        let points = polygon
            .vertices
            .iter()
            .map(|&v| mesh.vertices.get(v as usize).copied().map(lift))
            .collect::<Option<Vec<_>>>();
        let Some(points) = points else {
            continue;
        };
        match areas.iter_mut().find(|(area, _)| *area == polygon.area) {
            Some((_, polygons)) => polygons.push(points),
            None => areas.push((polygon.area, vec![points])),
        }
    }
    areas
}

#[cfg(test)]
mod tests {
    use panoply_nav::{NavPolygon, PrecinctSide};

    use super::*;

    /// A walkable square, a doorway and a walkable square in a row along x, the last on the
    /// precinct's +x side.
    fn row() -> NavMesh {
        let xs = [60., 61., 62., 64.];
        let vertices = xs
            .iter()
            .flat_map(|&x| [Vec3::new(x, 0., 0.), Vec3::new(x, 0., 1.)])
            .collect();
        // The square between x column `i` and `i + 1`, its edges from its -x side around.
        let square = |i: u32, edges: [NavEdge; 4], area| NavPolygon {
            vertices: vec![2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2],
            edges: edges.to_vec(),
            area,
            door: (area == NavAreaKind::Door).then_some(0),
        };
        use NavEdge::*;
        NavMesh {
            vertices,
            polygons: vec![
                square(0, [Wall, Wall, Neighbor(1), Wall], NavAreaKind::Walkable),
                square(1, [Neighbor(0), Wall, Neighbor(2), Wall], NavAreaKind::Door),
                square(
                    2,
                    [Neighbor(1), Wall, Boundary(PrecinctSide::PosX), Wall],
                    NavAreaKind::Walkable,
                ),
            ],
            doors: None,
        }
    }

    #[test]
    fn the_row_is_a_good_mesh() {
        assert_eq!(row().check(), Ok(()));
    }

    #[test]
    fn a_shared_edge_is_drawn_once() {
        let mesh = row();
        let edges: Vec<_> = edges(&mesh).collect();
        // Twelve sides, two of them shared.
        assert_eq!(edges.len(), 10);
        let neighbors = edges
            .iter()
            .filter(|(_, _, e)| matches!(e, NavEdge::Neighbor(_)));
        assert_eq!(neighbors.count(), 2);
        let boundary: Vec<_> = edges
            .iter()
            .filter(|(_, _, e)| matches!(e, NavEdge::Boundary(_)))
            .collect();
        assert_eq!(boundary.len(), 1);
        assert!(boundary[0].0.x == 64. && boundary[0].1.x == 64.);
    }

    #[test]
    fn polygons_are_filled_by_area() {
        let fills = fills(&row());
        let counts: Vec<_> = fills.iter().map(|(area, p)| (*area, p.len())).collect();
        assert_eq!(counts, [(NavAreaKind::Walkable, 2), (NavAreaKind::Door, 1)]);
        assert!(fills[0].1[0].iter().all(|p| p.y == FILL_LIFT));
    }

    #[test]
    fn a_missing_vertex_drops_only_its_polygon_and_edges() {
        let mut mesh = row();
        mesh.polygons[1].vertices[2] = 99;
        assert_eq!(fills(&mesh)[0].1.len(), 2);
        assert_eq!(fills(&mesh).len(), 1);
        // The doorway's edges into and out of vertex 99.
        assert_eq!(edges(&mesh).count(), 8);
    }
}
