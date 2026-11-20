//! A precinct's navigation mesh built by clipping polygons, a port of faery's builder
//! (`NavigationMeshBuilder.ts`), chosen over Recast (D99).
//!
//! Each walkable surface is a layer: a plane and the 2D outline of where it lies, the ground's
//! triangles joined wherever they share a plane. From each layer, the obstacles' footprints are
//! subtracted, each grown by the agent's radius and cut to the band of the layer the obstacle
//! blocks; doors, portals and footpaths split what is left by area; and each piece is divided into convex
//! polygons. Where the polygons of separate pieces meet, each edge is split at the other side's
//! vertices, so neighbours share their edges' vertices exactly.
//!
//! All of it is in one fixed-point space ([`UNITS_PER_METER`]), so a vertex two operations both
//! produce lands on the same point in each.

use std::collections::HashMap;

use bevy::math::{IVec2, Quat, Vec2, Vec3, Vec3Swizzles};
use i_overlay::{
    core::{
        extract::BooleanExtractionBuffer, fill_rule::FillRule, overlay::Overlay,
        overlay_rule::OverlayRule,
    },
    i_float::int::point::IntPoint,
    i_shape::int::shape::{IntContour, IntShapes},
};
use i_triangle::int::triangulatable::IntTriangulatable;
use panoply_core::{
    metrics::{PRECINCT_SIZE, PRECINCT_SIZE_F},
    ToBevy,
};
use panoply_scenery::{ColliderShape, ColliderType};

use crate::{
    agent::{placed_triangles, AGENT_HEIGHT, AGENT_RADIUS, WALKABLE_CLIMB, WALKABLE_NORMAL_Y},
    NavAreaKind, NavCollider, NavEdge, NavInput, NavMesh, NavPolygon, PrecinctSide, SurfaceKind,
};

/// The fixed-point grid every outline is cut on: about a millimeter.
const UNITS_PER_METER: f32 = 1024.;

/// A precinct's side in fixed-point units.
const PRECINCT_UNITS: i32 = PRECINCT_SIZE * UNITS_PER_METER as i32;

/// Ground heights are rounded to a quarter meter, as faery's were, so gentle ground falls into
/// fewer planes.
const GROUND_HEIGHT_STEP: f32 = 0.25;

/// Ground this deep is under water and not walked: faery's `MAX_WATER_DEPTH`, 0.7 m below the
/// water's surface (`WATER_HEIGHT`). Faery applied it only to parcels with water; the input does
/// not say which have it.
const DEEP_GROUND: f32 = -1.1;

/// How far a rounded box's footprint is cut back at each corner, along each side: faery's
/// `RBOX_BEVEL`.
const RBOX_BEVEL: f32 = 0.2;

/// Half the width of a footpath's area about its cells' middles, faery's `PATH_WIDTH`.
const PATH_HALF_WIDTH: f32 = 0.1;

/// How far apart two surfaces' heights may be at one point and still count as one height, where a
/// floor or walkable top covers the ground.
const HEIGHT_TOLERANCE: f32 = 0.01;

/// How far apart two surfaces' heights may be where their edges meet and still join, sharing the
/// first surface's vertices: half a climb, so a floor raised a little above the ground (0.02 m)
/// meets it. Faery joined these by rounding a floor's height to the meter.
const JOIN_TOLERANCE: f32 = WALKABLE_CLIMB / 2.;

/// Twice the smallest area a polygon keeps, in square meters: anything thinner is a sliver the
/// mesh check would refuse.
const MIN_DOUBLE_AREA: f32 = 2e-4;

/// Builds `input`'s navigation mesh.
pub fn build(input: &NavInput) -> NavMesh {
    let layers = layers(input);
    let covers = covers(&layers);
    let obstacles = obstacles(input);
    let doors = areas(input, ColliderType::Door);
    let portals = areas(input, ColliderType::Portal);
    let paths = footpaths(input);

    let mut pieces = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        let cut = |marks: &mut dyn Iterator<Item = &Mark>| -> Vec<IntContour<i32>> {
            marks
                .filter(|m| m.overlaps(layer))
                .filter_map(|m| m.on(&layer.plane))
                .collect()
        };
        let mut blocking = cut(&mut obstacles
            .iter()
            .filter(|o| !(o.walkable && layer.source == Source::Top))
            .map(|o| &o.mark));
        blocking.extend(
            covers
                .iter()
                .filter(|c| c.layer != index && c.mark.overlaps(layer))
                .filter_map(|c| {
                    let top_less = match (c.source, layer.source) {
                        (Source::Ground, Source::Ground) => return None,
                        (Source::Ground, _) => HEIGHT_TOLERANCE,
                        (_, Source::Ground) => -HEIGHT_TOLERANCE,
                        _ if c.layer < index => -HEIGHT_TOLERANCE,
                        _ => HEIGHT_TOLERANCE,
                    };
                    c.mark.on_less(&layer.plane, top_less)
                }),
        );
        let mut walkable = overlay(&layer.triangles, &blocking, OverlayRule::Difference);
        let mut split = |walkable: &mut IntShapes<i32>, marks: Vec<IntContour<i32>>, area| {
            if marks.is_empty() || walkable.is_empty() {
                return;
            }
            let (marked, rest) = split_by(walkable, &marks);
            *walkable = rest;
            pieces.push((marked, layer.plane, area));
        };
        split(&mut walkable, cut(&mut doors.iter()), NavAreaKind::Door);
        split(&mut walkable, cut(&mut portals.iter()), NavAreaKind::Portal);
        if layer.source == Source::Ground {
            let paths = paths
                .iter()
                .filter(|p| layer.overlaps_box(p.min, p.max))
                .map(|p| p.contour.clone())
                .collect();
            split(&mut walkable, paths, NavAreaKind::Path);
        }
        pieces.push((walkable, layer.plane, NavAreaKind::Walkable));
    }

    let mut builder = MeshBuilder::default();
    for (shapes, plane, area) in pieces {
        if shapes.is_empty() {
            continue;
        }
        for contour in shapes.triangulate().into_delaunay().to_convex_polygons() {
            builder.add(&contour, &plane, area);
        }
    }
    builder.finish()
}

/// A surface's plane, as a height over x and z.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Plane {
    /// The height's change per meter of x and of z, and its value at the origin.
    dx: f32,
    dz: f32,
    y0: f32,
}

impl Plane {
    fn flat(y: f32) -> Self {
        Self {
            dx: 0.,
            dz: 0.,
            y0: y,
        }
    }

    /// The plane through a triangle, if it faces up steeply enough to walk on, whichever way it
    /// winds.
    fn walkable(a: Vec3, b: Vec3, c: Vec3) -> Option<Self> {
        let mut normal = (b - a).cross(c - a);
        if normal.y < 0. {
            normal = -normal;
        }
        let normal = normal.try_normalize()?;
        (normal.y >= WALKABLE_NORMAL_Y).then(|| {
            let (dx, dz) = (-normal.x / normal.y, -normal.z / normal.y);
            Self {
                dx,
                dz,
                y0: a.y - dx * a.x - dz * a.z,
            }
        })
    }

    fn height(&self, p: Vec2) -> f32 {
        self.y0 + self.dx * p.x + self.dz * p.y
    }

    /// The plane rounded, so surfaces in one plane share a key though their floats differ in the
    /// last bits.
    fn key(&self) -> (i32, i32, i32) {
        let round = |v: f32| (v * 1e4).round() as i32;
        (round(self.dx), round(self.dz), round(self.y0))
    }
}

/// What a layer's surface is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Source {
    /// The ground, which footpaths mark.
    Ground,
    Floor,
    /// The top of a collider marked walkable.
    Top,
}

/// A walkable surface: a plane and the triangles of it that are walkable.
struct Layer {
    plane: Plane,
    source: Source,
    triangles: Vec<IntContour<i32>>,
    min: Vec2,
    max: Vec2,
}

impl Layer {
    fn overlaps_box(&self, min: Vec2, max: Vec2) -> bool {
        min.x < self.max.x && max.x > self.min.x && min.y < self.max.y && max.y > self.min.y
    }
}

/// The input's walkable surfaces, one layer per plane: the ground, the floors' tops, and the tops
/// of colliders marked walkable.
fn layers(input: &NavInput) -> Vec<Layer> {
    let mut layers: Vec<Layer> = Vec::new();
    let mut index: HashMap<(Source, (i32, i32, i32)), usize> = HashMap::new();
    let mut add = |points: [Vec3; 3], source: Source| {
        let Some(plane) = Plane::walkable(points[0], points[1], points[2]) else {
            return;
        };
        let outline = clip_to_precinct(points.iter().map(|p| p.xz()).collect());
        if outline.len() < 3 {
            return;
        }
        let contour = to_contour(&outline);
        let slot = *index.entry((source, plane.key())).or_insert_with(|| {
            layers.push(Layer {
                plane,
                source,
                triangles: Vec::new(),
                min: Vec2::MAX,
                max: Vec2::MIN,
            });
            layers.len() - 1
        });
        let layer = &mut layers[slot];
        for p in &outline {
            layer.min = layer.min.min(*p);
            layer.max = layer.max.max(*p);
        }
        layer.triangles.push(contour);
    };

    for surface in &input.surfaces {
        let ground = surface.kind == SurfaceKind::Ground;
        for &[a, b, c] in &surface.indices {
            let mut points = [a, b, c].map(|i| surface.vertices[i as usize]);
            if ground {
                for p in &mut points {
                    p.y = (p.y / GROUND_HEIGHT_STEP).round() * GROUND_HEIGHT_STEP;
                }
                if points.iter().any(|p| p.y <= DEEP_GROUND) {
                    continue;
                }
            }
            add(
                points,
                if ground {
                    Source::Ground
                } else {
                    Source::Floor
                },
            );
        }
    }
    for collider in input.colliders.iter().filter(|c| is_walkable(c)) {
        if let Some((vertices, indices)) = placed_triangles(collider) {
            for face in upper_faces(&vertices, &indices) {
                add(face, Source::Top);
            }
        }
    }
    layers
}

/// A convex shape's faces whose outward side faces up, judged against the shape's middle, since
/// its triangles' winding is not promised: a ramp's sloped face and not its underside.
fn upper_faces<'a>(
    vertices: &'a [Vec3],
    indices: &'a [[u32; 3]],
) -> impl Iterator<Item = [Vec3; 3]> + 'a {
    let middle = vertices.iter().copied().sum::<Vec3>() / vertices.len().max(1) as f32;
    indices
        .iter()
        .map(|t| t.map(|i| vertices[i as usize]))
        .filter(move |&[a, b, c]| {
            let mut normal = (b - a).cross(c - a);
            if normal.dot((a + b + c) / 3. - middle) < 0. {
                normal = -normal;
            }
            normal.y > 0.
        })
}

/// A layer's triangle, as what it covers on the layers under it.
struct Cover {
    layer: usize,
    source: Source,
    mark: Mark,
}

/// Each layer's triangles, covering any other layer under them within an agent's height: the
/// ground under a floor goes, as faery's `subtractShapeBeneath` took it, and a floor under the
/// ground. Where two meet at one height, only one keeps that part: a floor or a walkable top over
/// the ground, and between two others the one built first. The ground never lies over itself, so
/// it covers only the others. Each mark's `top_less` is set where it is applied (`build`).
fn covers(layers: &[Layer]) -> Vec<Cover> {
    let mut covers = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        for triangle in &layer.triangles {
            let outline: Vec<Vec2> = triangle.iter().map(|&p| to_meters(p)).collect();
            let lowest = outline
                .iter()
                .map(|&p| layer.plane.height(p))
                .fold(f32::MAX, f32::min);
            covers.extend(
                Mark::new(outline, lowest - AGENT_HEIGHT, layer.plane, 0.).map(|mark| Cover {
                    layer: index,
                    source: layer.source,
                    mark,
                }),
            );
        }
    }
    covers
}

fn is_walkable(collider: &NavCollider) -> bool {
    collider.collider.kind == ColliderType::Solid && collider.collider.walkable == Some(true)
}

/// A footprint that bears on the layers between two heights: an obstacle, or a door.
struct Mark {
    /// Counter-clockwise in x and z.
    outline: Vec<Vec2>,
    min: Vec2,
    max: Vec2,
    /// Which part of a layer under the outline it applies to: where the layer's height is above
    /// `bottom` and below `top` less `top_less`, `top` held to at most `ceiling` where one is set.
    bottom: f32,
    top: Plane,
    ceiling: Option<f32>,
    top_less: f32,
}

impl Mark {
    fn new(outline: Vec<Vec2>, bottom: f32, top: Plane, top_less: f32) -> Option<Self> {
        (outline.len() >= 3).then(|| {
            let min = outline.iter().copied().fold(Vec2::MAX, Vec2::min);
            let max = outline.iter().copied().fold(Vec2::MIN, Vec2::max);
            Self {
                outline,
                min,
                max,
                bottom,
                top,
                ceiling: None,
                top_less,
            }
        })
    }

    fn with_ceiling(self, ceiling: f32) -> Self {
        Self {
            ceiling: Some(ceiling),
            ..self
        }
    }

    fn overlaps(&self, layer: &Layer) -> bool {
        layer.overlaps_box(self.min, self.max)
    }

    /// The part of the outline where `layer` lies in this mark's band, as a contour; `None` where
    /// it lies outside it throughout. Both bounds are linear over the layer, so each cuts the
    /// outline along a line.
    fn on(&self, layer: &Plane) -> Option<IntContour<i32>> {
        self.on_less(layer, self.top_less)
    }

    /// [`Self::on`], the top lowered by `top_less` in place of the mark's own.
    fn on_less(&self, layer: &Plane, top_less: f32) -> Option<IntContour<i32>> {
        // Above the bottom: layer - bottom > 0.
        let above = Plane {
            y0: layer.y0 - self.bottom,
            ..*layer
        };
        // Below the top: top - top_less - layer > 0.
        let below = Plane {
            dx: self.top.dx - layer.dx,
            dz: self.top.dz - layer.dz,
            y0: self.top.y0 - top_less - layer.y0,
        };
        let mut outline = clip_half_plane(&clip_half_plane(&self.outline, &above), &below);
        if let Some(ceiling) = self.ceiling {
            // Below the ceiling too: ceiling - top_less - layer > 0.
            let under = Plane {
                dx: -layer.dx,
                dz: -layer.dz,
                y0: ceiling - top_less - layer.y0,
            };
            outline = clip_half_plane(&outline, &under);
        }
        (double_area(&outline) > MIN_DOUBLE_AREA).then(|| to_contour(&outline))
    }
}

/// What blocks a layer: solids and hints, each grown by the agent's radius and a solid by its
/// margin too. A mark blocks where a layer lies from an agent's height below its bottom to a climb
/// below its top: a body that stands there hits it, and one that stands above it steps up. A
/// walkable collider's top is its walkable face, so the ramp it is lets the ground meet its foot,
/// held to its highest point, so past its top edge it does not rise over the floor it climbs to;
/// an unwalkable one's top is flat across. Floors are not obstacles, as in faery: a
/// floor's cover takes the ground under it, and the ground beside a raised floor stays.
fn obstacles(input: &NavInput) -> Vec<Obstacle> {
    let mut obstacles = Vec::new();
    for collider in &input.colliders {
        let desc = &collider.collider;
        let grow = match desc.kind {
            ColliderType::Solid => AGENT_RADIUS + desc.margin.unwrap_or(0.).max(0.),
            ColliderType::Hint => AGENT_RADIUS,
            _ => continue,
        };
        let Some((vertices, indices)) = placed_triangles(collider) else {
            continue;
        };
        let Some(outline) = footprint(collider, grow) else {
            continue;
        };
        let (bottom, highest) = height_range(&vertices);
        let top = is_walkable(collider)
            .then(|| top_face(&vertices, &indices))
            .flatten()
            .unwrap_or(Plane::flat(highest));
        obstacles.extend(
            Mark::new(outline, bottom - AGENT_HEIGHT, top, WALKABLE_CLIMB).map(|mark| Obstacle {
                walkable: is_walkable(collider),
                mark: mark.with_ceiling(highest),
            }),
        );
    }
    obstacles
}

/// An obstacle's mark, and whether its collider is walkable: one that is does not block the tops
/// of walkable colliders, so the ramps and platforms of one piece join, as faery's ramps did not
/// cut ramps (`subtractCollider`). It still blocks the ground and floors.
struct Obstacle {
    walkable: bool,
    mark: Mark,
}

/// A collider's outline seen from above, grown by `grow` on every side, as faery cut it
/// (`subtractCollider`): a box or a ramp as its rectangle, square-cornered; a rounded box beveled
/// at its corners; and a round shape as an octagon about its radii.
fn footprint(collider: &NavCollider, grow: f32) -> Option<Vec<Vec2>> {
    let (_, pose) = collider.collider.volume()?;
    let rectangle = |x: f32, z: f32| vec![(-x, -z), (x, -z), (x, z), (-x, z)];
    let beveled = |x: f32, z: f32, bx: f32, bz: f32| {
        vec![
            (-x + bx, -z),
            (x - bx, -z),
            (x, -z + bz),
            (x, z - bz),
            (x - bx, z),
            (-x + bx, z),
            (-x, z - bz),
            (-x, -z + bz),
        ]
    };
    // Faery's octagon (`makeCircleClip`) bevels each corner by half the half-width.
    let octagon = |x: f32, z: f32| beveled(x, z, x / 2., z / 2.);
    let local = match collider.collider.shape {
        ColliderShape::Box { half_extents } | ColliderShape::Ramp { half_extents, .. } => {
            rectangle(half_extents.x + grow, half_extents.z + grow)
        }
        ColliderShape::Rbox { half_extents } => beveled(
            half_extents.x + grow,
            half_extents.z + grow,
            RBOX_BEVEL,
            RBOX_BEVEL,
        ),
        ColliderShape::Cylinder { radius, .. } | ColliderShape::Sphere { radius } => {
            octagon(radius + grow, radius + grow)
        }
        ColliderShape::Ellipsoid { radii } => octagon(radii.x + grow, radii.z + grow),
    };
    let (offset, turn) = (pose.translation.to_bevy(), pose.rotation.to_bevy());
    let facing = Quat::from_rotation_y(collider.facing);
    Some(
        local
            .into_iter()
            .map(|(x, z)| (collider.position + facing * (offset + turn * Vec3::new(x, 0., z))).xz())
            .collect(),
    )
}

/// The colliders of `kind` as areas, as faery took a door's closed leaf and a portal's aperture:
/// each one's footprint, on the layers within a climb of its bottom.
fn areas(input: &NavInput, kind: ColliderType) -> Vec<Mark> {
    input
        .colliders
        .iter()
        .filter(|c| c.collider.kind == kind)
        .filter_map(|door| {
            let (vertices, _) = placed_triangles(door)?;
            let (bottom, _) = height_range(&vertices);
            let outline = footprint(door, 0.)?;
            Mark::new(
                outline,
                bottom - WALKABLE_CLIMB,
                Plane::flat(bottom + WALKABLE_CLIMB),
                0.,
            )
        })
        .collect()
}

/// The lowest and highest of `points`.
fn height_range(points: &[Vec3]) -> (f32, f32) {
    points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    })
}

/// The plane of a shape's largest walkable face, seen from above.
fn top_face(vertices: &[Vec3], indices: &[[u32; 3]]) -> Option<Plane> {
    upper_faces(vertices, indices)
        .filter_map(|[a, b, c]| {
            let plane = Plane::walkable(a, b, c)?;
            Some(((b - a).xz().perp_dot((c - a).xz()).abs(), plane))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, plane)| plane)
}

/// A footpath cell's outline.
struct PathOutline {
    contour: IntContour<i32>,
    min: Vec2,
    max: Vec2,
}

/// Each footpath cell's outline, as faery's `addTerrainPaths` cut it: [`PATH_HALF_WIDTH`] about
/// the cell's middle lines, reaching out to each side whose neighbour is a path, and filling a
/// corner where both sides and the diagonal are; where both sides are but not the diagonal, the
/// corner is cut across.
fn footpaths(input: &NavInput) -> Vec<PathOutline> {
    let mut outlines = Vec::new();
    for z in 0..PRECINCT_SIZE {
        for x in 0..PRECINCT_SIZE {
            if !input.path_at(x, z) {
                continue;
            }
            let path = |dx: i32, dz: i32| input.path_at(x + dx, z + dz);
            let (x0, x3) = (x as f32, x as f32 + 1.);
            let (z0, z3) = (z as f32, z as f32 + 1.);
            let (x1, x2) = (x0 + 0.5 - PATH_HALF_WIDTH, x0 + 0.5 + PATH_HALF_WIDTH);
            let (z1, z2) = (z0 + 0.5 - PATH_HALF_WIDTH, z0 + 0.5 + PATH_HALF_WIDTH);

            // Each corner in turn around the cell, with the sides that meet there: the side
            // arrived along, then the side left along.
            let corners = [
                (
                    (-1, -1),
                    (x0, z0),
                    (x1, z1),
                    [((-1, 0), (x0, z1)), ((0, -1), (x1, z0))],
                ),
                (
                    (1, -1),
                    (x3, z0),
                    (x2, z1),
                    [((0, -1), (x2, z0)), ((1, 0), (x3, z1))],
                ),
                (
                    (1, 1),
                    (x3, z3),
                    (x2, z2),
                    [((1, 0), (x3, z2)), ((0, 1), (x2, z3))],
                ),
                (
                    (-1, 1),
                    (x0, z3),
                    (x1, z2),
                    [((0, 1), (x1, z3)), ((-1, 0), (x0, z2))],
                ),
            ];
            let mut outline = Vec::new();
            for ((dx, dz), outer, inner, sides) in corners {
                if path(dx, 0) && path(0, dz) && path(dx, dz) {
                    outline.push(Vec2::from(outer));
                    continue;
                }
                let open: Vec<_> = sides.iter().filter(|(d, _)| path(d.0, d.1)).collect();
                if open.is_empty() {
                    outline.push(Vec2::from(inner));
                }
                outline.extend(open.iter().map(|(_, p)| Vec2::from(*p)));
            }
            let min = outline.iter().copied().fold(Vec2::MAX, Vec2::min);
            let max = outline.iter().copied().fold(Vec2::MIN, Vec2::max);
            outlines.push(PathOutline {
                contour: to_contour(&outline),
                min,
                max,
            });
        }
    }
    outlines
}

/// `subject` against `clip` by `rule`, filled where any contour winds.
fn overlay(
    subject: &[IntContour<i32>],
    clip: &[IntContour<i32>],
    rule: OverlayRule,
) -> IntShapes<i32> {
    Overlay::from_subj_and_clip(subject, clip).overlay(rule, FillRule::NonZero)
}

/// `shapes` split into the part inside `marks` and the rest, both from one overlay, so the points
/// where the marks cross the shapes' outlines are the same on both sides.
fn split_by(
    shapes: &IntShapes<i32>,
    marks: &[IntContour<i32>],
) -> (IntShapes<i32>, IntShapes<i32>) {
    let subject = flatten(shapes);
    let mut overlay = Overlay::from_subj_and_clip(&subject, marks);
    let Some(graph) = overlay.build_graph_view(FillRule::NonZero) else {
        return (Vec::new(), Vec::new());
    };
    let mut buffer = BooleanExtractionBuffer::default();
    (
        graph.extract_shapes(OverlayRule::Intersect, &mut buffer),
        graph.extract_shapes(OverlayRule::Difference, &mut buffer),
    )
}

/// Shapes as their contours, outer and holes alike: under the non-zero rule a hole's opposite
/// winding still cuts it out.
fn flatten(shapes: &IntShapes<i32>) -> Vec<IntContour<i32>> {
    shapes.iter().flatten().cloned().collect()
}

fn to_units(p: Vec2) -> IntPoint<i32> {
    let p = (p * UNITS_PER_METER).round();
    IntPoint::new(p.x as i32, p.y as i32)
}

fn to_meters(p: IntPoint<i32>) -> Vec2 {
    Vec2::new(p.x as f32, p.y as f32) / UNITS_PER_METER
}

/// An outline as a counter-clockwise contour.
fn to_contour(outline: &[Vec2]) -> IntContour<i32> {
    let mut contour: Vec<_> = outline.iter().map(|&p| to_units(p)).collect();
    contour.dedup();
    if double_area(outline) < 0. {
        contour.reverse();
    }
    contour
}

/// Twice an outline's area, positive when it is counter-clockwise in x and z.
fn double_area(outline: &[Vec2]) -> f32 {
    let n = outline.len();
    (0..n)
        .map(|i| outline[i].perp_dot(outline[(i + 1) % n]))
        .sum()
}

/// The part of a convex outline where `plane`'s height is positive (Sutherland–Hodgman).
fn clip_half_plane(outline: &[Vec2], plane: &Plane) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(outline.len() + 1);
    for (i, &a) in outline.iter().enumerate() {
        let b = outline[(i + 1) % outline.len()];
        let (ha, hb) = (plane.height(a), plane.height(b));
        if ha > 0. {
            out.push(a);
        }
        if (ha > 0.) != (hb > 0.) {
            out.push(a + (b - a) * (ha / (ha - hb)));
        }
    }
    out
}

/// A convex outline cut to the precinct's square.
fn clip_to_precinct(outline: Vec<Vec2>) -> Vec<Vec2> {
    let inside =
        |p: &Vec2| (0. ..=PRECINCT_SIZE_F).contains(&p.x) && (0. ..=PRECINCT_SIZE_F).contains(&p.y);
    if outline.iter().all(inside) {
        return outline;
    }
    let sides = [
        Plane {
            dx: 1.,
            dz: 0.,
            y0: 0.,
        },
        Plane {
            dx: -1.,
            dz: 0.,
            y0: PRECINCT_SIZE_F,
        },
        Plane {
            dx: 0.,
            dz: 1.,
            y0: 0.,
        },
        Plane {
            dx: 0.,
            dz: -1.,
            y0: PRECINCT_SIZE_F,
        },
    ];
    sides
        .iter()
        .fold(outline, |outline, side| clip_half_plane(&outline, side))
}

/// Gathers convex polygons into a [`NavMesh`], sharing vertices where they meet.
#[derive(Default)]
struct MeshBuilder {
    vertices: Vec<Vec3>,
    points: Vec<IntPoint<i32>>,
    /// Vertices by their fixed-point position, each with its height.
    at: HashMap<IntPoint<i32>, Vec<u32>>,
    /// Vertices by the meter cell they lie in.
    cells: HashMap<IVec2, Vec<u32>>,
    polygons: Vec<(Vec<u32>, NavAreaKind)>,
}

impl MeshBuilder {
    /// A counter-clockwise contour in x and z on `plane`, turned to run as the mesh's do.
    fn add(&mut self, contour: &[IntPoint<i32>], plane: &Plane, area: NavAreaKind) {
        let outline: Vec<Vec2> = contour.iter().map(|&p| to_meters(p)).collect();
        if double_area(&outline) <= MIN_DOUBLE_AREA {
            return;
        }
        let vertices = contour
            .iter()
            .rev()
            .map(|&p| self.vertex(p, plane.height(to_meters(p))))
            .collect();
        self.polygons.push((vertices, area));
    }

    fn vertex(&mut self, point: IntPoint<i32>, y: f32) -> u32 {
        let same = self.at.entry(point).or_default();
        if let Some(&v) = same
            .iter()
            .find(|&&v| (self.vertices[v as usize].y - y).abs() <= JOIN_TOLERANCE)
        {
            return v;
        }
        let index = self.vertices.len() as u32;
        same.push(index);
        let meters = to_meters(point);
        self.vertices.push(Vec3::new(meters.x, y, meters.y));
        self.points.push(point);
        self.cells
            .entry(meters.floor().as_ivec2())
            .or_default()
            .push(index);
        index
    }

    /// Splits each edge at the vertices other polygons have along it, then names what lies
    /// across each edge.
    fn finish(mut self) -> NavMesh {
        let polygons = std::mem::take(&mut self.polygons);
        let polygons: Vec<(Vec<u32>, NavAreaKind)> = polygons
            .into_iter()
            .map(|(vertices, area)| (self.split_edges(&vertices), area))
            .collect();

        let mut by_edge: HashMap<(u32, u32), u32> = HashMap::new();
        for (index, (vertices, _)) in polygons.iter().enumerate() {
            for (a, b) in edges(vertices) {
                by_edge.insert((a, b), index as u32);
            }
        }
        let polygons = polygons
            .into_iter()
            .map(|(vertices, area)| {
                let edges = edges(&vertices)
                    .map(|(a, b)| match by_edge.get(&(b, a)) {
                        Some(&other) => NavEdge::Neighbor(other),
                        None => self.side(a, b).map_or(NavEdge::Wall, NavEdge::Boundary),
                    })
                    .collect();
                NavPolygon {
                    vertices,
                    edges,
                    area,
                }
            })
            .collect();
        NavMesh {
            vertices: self.vertices,
            polygons,
        }
    }

    /// A polygon's vertices with, along each edge, every other vertex lying exactly on it at its
    /// height, in order.
    fn split_edges(&self, vertices: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(vertices.len());
        for (a, b) in edges(vertices) {
            out.push(a);
            let (pa, pb) = (self.points[a as usize], self.points[b as usize]);
            let (ya, yb) = (self.vertices[a as usize].y, self.vertices[b as usize].y);
            let (dx, dy) = ((pb.x - pa.x) as i64, (pb.y - pa.y) as i64);
            let length = dx * dx + dy * dy;
            let (lo, hi) = {
                let (ma, mb) = (to_meters(pa).floor(), to_meters(pb).floor());
                (ma.min(mb).as_ivec2(), ma.max(mb).as_ivec2())
            };
            let mut between: Vec<(i64, u32)> = Vec::new();
            for cz in lo.y..=hi.y {
                for cx in lo.x..=hi.x {
                    let Some(cell) = self.cells.get(&IVec2::new(cx, cz)) else {
                        continue;
                    };
                    for &v in cell {
                        let p = self.points[v as usize];
                        let (ex, ey) = ((p.x - pa.x) as i64, (p.y - pa.y) as i64);
                        let along = ex * dx + ey * dy;
                        if dx * ey - dy * ex != 0 || along <= 0 || along >= length {
                            continue;
                        }
                        let t = along as f32 / length as f32;
                        let y = ya + (yb - ya) * t;
                        if (self.vertices[v as usize].y - y).abs() <= JOIN_TOLERANCE {
                            between.push((along, v));
                        }
                    }
                }
            }
            between.sort_unstable();
            out.extend(between.into_iter().map(|(_, v)| v));
        }
        out
    }

    /// The precinct side both vertices lie on, if any.
    fn side(&self, a: u32, b: u32) -> Option<PrecinctSide> {
        let (pa, pb) = (self.points[a as usize], self.points[b as usize]);
        [
            (pa.x == 0 && pb.x == 0, PrecinctSide::NegX),
            (
                pa.x == PRECINCT_UNITS && pb.x == PRECINCT_UNITS,
                PrecinctSide::PosX,
            ),
            (pa.y == 0 && pb.y == 0, PrecinctSide::NegZ),
            (
                pa.y == PRECINCT_UNITS && pb.y == PRECINCT_UNITS,
                PrecinctSide::PosZ,
            ),
        ]
        .into_iter()
        .find_map(|(on, side)| on.then_some(side))
    }
}

/// A polygon's edges, each as its two vertices, the last back to the first.
fn edges(vertices: &[u32]) -> impl Iterator<Item = (u32, u32)> + '_ {
    (0..vertices.len()).map(|i| (vertices[i], vertices[(i + 1) % vertices.len()]))
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec3;
    use panoply_scenery::{ColliderDesc, ColliderShape};

    use super::*;
    use crate::NavSurface;

    /// Flat ground over the whole precinct, as two triangles.
    fn flat_ground() -> NavSurface {
        let s = PRECINCT_SIZE_F;
        NavSurface {
            kind: SurfaceKind::Ground,
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(s, 0., 0.),
                Vec3::new(s, 0., s),
                Vec3::new(0., 0., s),
            ],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        }
    }

    fn post(x: f32, z: f32) -> NavCollider {
        NavCollider {
            position: Vec3::new(x, 0., z),
            facing: 0.,
            collider: ColliderDesc {
                shape: ColliderShape::Box {
                    half_extents: Vec3::new(0.5, 1., 0.5),
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn open_ground_meets_every_side() {
        let input = NavInput {
            surfaces: vec![flat_ground()],
            ..Default::default()
        };
        let mesh = build(&input);
        assert_eq!(mesh.check(), Ok(()));
        let sides: Vec<_> = mesh
            .polygons
            .iter()
            .flat_map(|p| &p.edges)
            .filter_map(|e| match e {
                NavEdge::Boundary(side) => Some(*side),
                _ => None,
            })
            .collect();
        for side in [
            PrecinctSide::NegX,
            PrecinctSide::PosX,
            PrecinctSide::NegZ,
            PrecinctSide::PosZ,
        ] {
            assert!(sides.contains(&side), "{side:?}");
        }
    }

    #[test]
    fn a_post_is_cut_out_with_the_agent_radius() {
        let input = NavInput {
            surfaces: vec![flat_ground()],
            colliders: vec![post(32., 32.)],
            ..Default::default()
        };
        let mesh = build(&input);
        assert_eq!(mesh.check(), Ok(()));
        // Nothing within the post grown by the agent's radius, short of the circumscribed ring.
        let reach = 0.5 + AGENT_RADIUS * 0.98;
        for polygon in &mesh.polygons {
            let n = polygon.vertices.len() as f32;
            let centroid = polygon
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize].xz())
                .sum::<Vec2>()
                / n;
            let off = (centroid - Vec2::splat(32.)).abs();
            assert!(off.x > reach || off.y > reach, "{centroid}");
        }
        // The polygons around the hole are joined.
        assert!(mesh
            .polygons
            .iter()
            .flat_map(|p| &p.edges)
            .any(|e| matches!(e, NavEdge::Neighbor(_))));
    }

    #[test]
    fn a_low_step_does_not_block() {
        let mut step = post(32., 32.);
        step.collider.shape = ColliderShape::Box {
            half_extents: Vec3::new(0.5, WALKABLE_CLIMB * 0.4, 0.5),
        };
        let input = NavInput {
            surfaces: vec![flat_ground()],
            colliders: vec![step],
            ..Default::default()
        };
        let area = total_area(&build(&input));
        assert!(
            (area - PRECINCT_SIZE_F * PRECINCT_SIZE_F).abs() < 0.01,
            "{area}"
        );
    }

    /// The polygons' area seen from above, overlaps counted each time.
    fn total_area(mesh: &NavMesh) -> f32 {
        mesh.polygons
            .iter()
            .map(|p| {
                let outline: Vec<Vec2> = p
                    .vertices
                    .iter()
                    .map(|&v| mesh.vertices[v as usize].xz())
                    .collect();
                -double_area(&outline) / 2.
            })
            .sum()
    }

    /// A floor raised a little over the ground covers the ground under it, so nothing lies
    /// stacked there, and its edges join the ground's.
    #[test]
    fn a_floor_covers_the_ground_and_joins_it() {
        let y = 0.02;
        let floor = NavSurface {
            kind: SurfaceKind::Floor,
            vertices: vec![
                Vec3::new(30., y, 30.),
                Vec3::new(34., y, 30.),
                Vec3::new(34., y, 34.),
                Vec3::new(30., y, 34.),
            ],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let input = NavInput {
            surfaces: vec![flat_ground(), floor],
            ..Default::default()
        };
        let mesh = build(&input);
        assert_eq!(mesh.check(), Ok(()));
        // Stacked, the floor's area would count twice.
        let area = total_area(&mesh);
        assert!(
            (area - PRECINCT_SIZE_F * PRECINCT_SIZE_F).abs() < 0.01,
            "{area}"
        );
        // No edge is a wall: the floor joins the ground all round.
        assert!(mesh
            .polygons
            .iter()
            .flat_map(|p| &p.edges)
            .all(|e| *e != NavEdge::Wall));
    }

    /// A stair 1 m wide from the ground at z 30 up 2 m to a floor beginning at z 32 joins the
    /// floor along its top edge, as `MossyStoneStairsTall` meets the tier above.
    #[test]
    fn a_stair_joins_the_floor_at_its_top() {
        let stair = NavCollider {
            position: Vec3::new(32., 0., 31.),
            facing: 0.,
            collider: ColliderDesc {
                shape: ColliderShape::Ramp {
                    half_extents: Vec3::new(0.5, 1., 1.),
                    facing: 0.,
                },
                walkable: Some(true),
                ..Default::default()
            },
        };
        let floor = NavSurface {
            kind: SurfaceKind::Floor,
            vertices: vec![
                Vec3::new(28., 2., 32.),
                Vec3::new(36., 2., 32.),
                Vec3::new(36., 2., 36.),
                Vec3::new(28., 2., 36.),
            ],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let input = NavInput {
            surfaces: vec![flat_ground(), floor],
            colliders: vec![stair],
            ..Default::default()
        };
        let mesh = build(&input);
        assert_eq!(mesh.check(), Ok(()));
        // The edges along the stair's top, between its sides.
        let top: Vec<NavEdge> = mesh
            .polygons
            .iter()
            .flat_map(|p| edges(&p.vertices).zip(p.edges.iter().copied()))
            .filter(|&((a, b), _)| {
                let (a, b) = (mesh.vertices[a as usize], mesh.vertices[b as usize]);
                let on_top = |v: Vec3| (v.z - 32.).abs() < 1e-3 && (v.y - 2.).abs() < 0.1;
                let between = |v: Vec3| (31.49..=32.51).contains(&v.x);
                on_top(a) && on_top(b) && between(a) && between(b)
            })
            .map(|(_, e)| e)
            .collect();
        assert!(!top.is_empty(), "nothing lies along the stair's top");
        assert!(
            top.iter().all(|e| matches!(e, NavEdge::Neighbor(_))),
            "{top:?}"
        );
        // The floor is whole: nothing past the stair's top edge is cut from it.
        let floor = NavMesh {
            vertices: mesh.vertices.clone(),
            polygons: mesh
                .polygons
                .iter()
                .filter(|p| {
                    p.vertices
                        .iter()
                        .all(|&v| (mesh.vertices[v as usize].y - 2.).abs() < 0.1)
                })
                .cloned()
                .collect(),
        };
        let area = total_area(&floor);
        assert!((area - 32.).abs() < 0.01, "{area}");
    }

    /// A ramp up to a walkable platform, both parts of one piece as the stone portal's are, joins
    /// it along the platform's edge, and nothing on the ramp is cut off from the rest.
    #[test]
    fn a_ramp_joins_its_platform() {
        let part = |z: f32, shape| NavCollider {
            position: Vec3::new(32., 0., 32.),
            facing: 0.,
            collider: ColliderDesc {
                shape,
                offset: Some(Vec3::new(0., 0., z)),
                walkable: Some(true),
                ..Default::default()
            },
        };
        let half_extents = Vec3::new(0.5, 0.4, 0.5);
        let input = NavInput {
            surfaces: vec![flat_ground()],
            colliders: vec![
                part(
                    0.,
                    ColliderShape::Ramp {
                        half_extents,
                        facing: 0.,
                    },
                ),
                part(1., ColliderShape::Box { half_extents }),
            ],
            ..Default::default()
        };
        let mesh = build(&input);
        assert_eq!(mesh.check(), Ok(()));
        let raised: Vec<&NavPolygon> = mesh
            .polygons
            .iter()
            .filter(|p| {
                p.vertices
                    .iter()
                    .any(|&v| mesh.vertices[v as usize].y > 0.1)
            })
            .collect();
        assert!(!raised.is_empty());
        for polygon in raised {
            assert!(
                polygon
                    .edges
                    .iter()
                    .any(|e| matches!(e, NavEdge::Neighbor(_))),
                "an isolated polygon on the ramp or platform"
            );
        }
        // The edges where the ramp's top meets the platform, at z 32.5.
        let seam: Vec<NavEdge> = mesh
            .polygons
            .iter()
            .flat_map(|p| edges(&p.vertices).zip(p.edges.iter().copied()))
            .filter(|&((a, b), _)| {
                let at = |v: u32| {
                    let v = mesh.vertices[v as usize];
                    (v.z - 32.5).abs() < 1e-3 && (v.y - 0.8).abs() < 0.1
                };
                at(a) && at(b)
            })
            .map(|(_, e)| e)
            .collect();
        assert!(!seam.is_empty(), "nothing meets the platform's edge");
        assert!(
            seam.iter().all(|e| matches!(e, NavEdge::Neighbor(_))),
            "{seam:?}"
        );
    }

    #[test]
    fn half_planes_cut_convex_outlines() {
        let square = vec![
            Vec2::new(0., 0.),
            Vec2::new(2., 0.),
            Vec2::new(2., 2.),
            Vec2::new(0., 2.),
        ];
        // x < 1
        let cut = clip_half_plane(
            &square,
            &Plane {
                dx: -1.,
                dz: 0.,
                y0: 1.,
            },
        );
        assert!((double_area(&cut) - 4.).abs() < 1e-5);
    }
}
