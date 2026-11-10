//! Flat shapes, stroked or filled, drawn in a plane and turned into triangles.

use bevy::{
    math::{Rect, Vec2, Vec3},
    mesh::{Indices, Mesh},
};

/// A marker at an open end of a stroked polyline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StrokeMarker {
    #[default]
    None,
    Arrowhead,
}

/// How to stroke a polygon or polyline.
#[derive(Clone, Debug)]
pub struct PolygonOptions {
    /// Joins the last point back to the first. A polygon needs at least three distinct points.
    pub closed: bool,
    /// Length of each dash; the default, `f32::MAX`, is a solid line.
    pub dash_length: f32,
    /// Length of the gap between dashes.
    pub gap_length: f32,
    /// Markers at the start and end of an open polyline.
    pub start_marker: StrokeMarker,
    pub end_marker: StrokeMarker,
}

impl Default for PolygonOptions {
    fn default() -> Self {
        Self {
            closed: false,
            dash_length: f32::MAX,
            gap_length: 0.,
            start_marker: StrokeMarker::None,
            end_marker: StrokeMarker::None,
        }
    }
}

/// The axis-aligned plane shapes are drawn in: the 2D coordinates `(x, y)` and the offset `z` go
/// to the named world axes, in order.
#[derive(Copy, Clone, Debug, Default)]
pub enum DrawPlane {
    #[default]
    XY,
    /// The ground: `x` is world x, `y` is world z, and the offset is height.
    XZ,
    YX,
    YZ,
    ZX,
    ZY,
}

impl DrawPlane {
    pub fn project(&self, x: f32, y: f32, z: f32) -> Vec3 {
        match self {
            DrawPlane::XY => Vec3::new(x, y, z),
            DrawPlane::XZ => Vec3::new(x, z, y),
            DrawPlane::YX => Vec3::new(y, x, z),
            DrawPlane::YZ => Vec3::new(y, z, x),
            DrawPlane::ZX => Vec3::new(z, x, y),
            DrawPlane::ZY => Vec3::new(z, y, x),
        }
    }
}

/// How far a mitered corner may reach, in half stroke widths. A sharper join ends each segment
/// square instead of drawing a spike.
const MITER_LIMIT: f32 = 4.0;

/// Points closer than this are one point: a zero-length segment has no direction.
const MIN_SEGMENT: f32 = 1e-5;

/// Builds a mesh of flat shapes.
#[derive(Clone, Debug)]
pub struct ShapeBuilder {
    vertices: Vec<Vec3>,
    indices: Vec<u32>,
    stroke_width: f32,
    draw_plane: DrawPlane,
    offset: f32,
}

impl Default for ShapeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The left-hand perpendicular of a direction.
fn perp(dir: Vec2) -> Vec2 {
    Vec2::new(dir.y, -dir.x)
}

impl ShapeBuilder {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            stroke_width: 1.0,
            draw_plane: DrawPlane::default(),
            offset: 0.0,
        }
    }

    /// The width of strokes drawn from now on.
    pub fn with_stroke_width(&mut self, stroke_width: f32) -> &mut Self {
        self.stroke_width = stroke_width;
        self
    }

    /// The plane shapes are drawn in from now on.
    pub fn with_draw_plane(&mut self, draw_plane: DrawPlane) -> &mut Self {
        self.draw_plane = draw_plane;
        self
    }

    /// The distance from the draw plane of shapes drawn from now on.
    pub fn with_offset(&mut self, offset: f32) -> &mut Self {
        self.offset = offset;
        self
    }

    fn vertex(&mut self, p: Vec2) -> u32 {
        let index = self.vertices.len() as u32;
        self.vertices
            .push(self.draw_plane.project(p.x, p.y, self.offset));
        index
    }

    fn triangle(&mut self, a: Vec2, b: Vec2, c: Vec2) {
        let start = self.vertex(a);
        self.vertex(b);
        self.vertex(c);
        self.indices.extend([start, start + 1, start + 2]);
    }

    /// A quad from its corners in order around it.
    fn quad(&mut self, a: Vec2, b: Vec2, c: Vec2, d: Vec2) {
        let start = self.vertex(a);
        self.vertex(b);
        self.vertex(c);
        self.vertex(d);
        self.indices
            .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }

    /// A rectangle's outline, inside its bounds. A stroke too wide for it fills it.
    pub fn stroke_rect(&mut self, rect: Rect) -> &mut Self {
        if rect.is_empty() {
            return self;
        }
        let lw = self.stroke_width;
        if lw * 2. >= rect.width().min(rect.height()) {
            return self.fill_rect(rect);
        }
        let (min, max) = (rect.min, rect.max);
        self.fill_rect(Rect::new(min.x, min.y, max.x, min.y + lw));
        self.fill_rect(Rect::new(min.x, max.y - lw, max.x, max.y));
        self.fill_rect(Rect::new(min.x, min.y + lw, min.x + lw, max.y - lw));
        self.fill_rect(Rect::new(max.x - lw, min.y + lw, max.x, max.y - lw));
        self
    }

    pub fn fill_rect(&mut self, rect: Rect) -> &mut Self {
        if rect.is_empty() {
            return self;
        }
        self.quad(
            rect.min,
            Vec2::new(rect.max.x, rect.min.y),
            rect.max,
            Vec2::new(rect.min.x, rect.max.y),
        );
        self
    }

    /// Lines at every multiple of `spacing` strictly inside `rect`, both ways, centered on their
    /// positions; the border is not drawn.
    pub fn grid(&mut self, rect: Rect, spacing: f32) -> &mut Self {
        if rect.is_empty() || spacing <= 0. {
            return self;
        }
        let half = self.stroke_width * 0.5;
        let first = |min: f32| (min / spacing).floor() as i32 + 1;
        let last = |max: f32| (max / spacing).ceil() as i32 - 1;
        for i in first(rect.min.x)..=last(rect.max.x) {
            let x = i as f32 * spacing;
            self.fill_rect(Rect::new(x - half, rect.min.y, x + half, rect.max.y));
        }
        for i in first(rect.min.y)..=last(rect.max.y) {
            let y = i as f32 * spacing;
            self.fill_rect(Rect::new(rect.min.x, y - half, rect.max.x, y + half));
        }
        self
    }

    /// A ring inside `radius`.
    pub fn stroke_circle(&mut self, center: Vec2, radius: f32, segments: u32) -> &mut Self {
        if segments < 3 {
            return self;
        }
        let inner = (radius - self.stroke_width).max(0.0);
        let step = std::f32::consts::TAU / segments as f32;
        let point = |i: u32, r: f32| center + Vec2::from_angle(i as f32 * step) * r;
        for i in 0..segments {
            self.quad(
                point(i, inner),
                point(i, radius),
                point(i + 1, radius),
                point(i + 1, inner),
            );
        }
        self
    }

    pub fn fill_circle(&mut self, center: Vec2, radius: f32, segments: u32) -> &mut Self {
        if segments < 3 {
            return self;
        }
        let start = self.vertex(center);
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            self.vertex(center + Vec2::from_angle(i as f32 * step) * radius);
        }
        for i in 0..segments {
            let next = (i + 1) % segments;
            self.indices
                .extend([start, start + 1 + i, start + 1 + next]);
        }
        self
    }

    pub fn fill_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2) -> &mut Self {
        self.triangle(a, b, c);
        self
    }

    /// A quad from its corners in order around it.
    pub fn fill_quad(&mut self, a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> &mut Self {
        self.quad(a, b, c, d);
        self
    }

    /// Strokes a polyline, or with `options.closed` a polygon, centered on its points: mitered
    /// where segments join, square at open ends, dashed if `options` say so.
    pub fn stroke_polygon(&mut self, points: &[Vec2], options: PolygonOptions) -> &mut Self {
        let mut pts: Vec<Vec2> = Vec::with_capacity(points.len());
        for &p in points {
            if pts.last().is_none_or(|q| q.distance(p) > MIN_SEGMENT) {
                pts.push(p);
            }
        }
        if options.closed && pts.len() > 2 && pts[0].distance(pts[pts.len() - 1]) <= MIN_SEGMENT {
            pts.pop();
        }
        if pts.len() < 2 {
            return self;
        }
        let closed = options.closed && pts.len() > 2;
        let n = pts.len();

        // Markers take their length from the ends of an open line.
        if !closed {
            let dir = (pts[1] - pts[0]).normalize();
            let length = self.marker_length(options.start_marker, pts[0].distance(pts[1]));
            if length > 0. {
                pts[0] += dir * length;
                self.fill_marker(options.start_marker, pts[0], -dir, length);
            }
            let dir = (pts[n - 1] - pts[n - 2]).normalize();
            let length = self.marker_length(options.end_marker, pts[n - 2].distance(pts[n - 1]));
            if length > 0. {
                pts[n - 1] -= dir * length;
                self.fill_marker(options.end_marker, pts[n - 1], dir, length);
            }
        }

        let segments = if closed { n } else { n - 1 };
        let dirs: Vec<Vec2> = (0..segments)
            .map(|k| (pts[(k + 1) % n] - pts[k]).normalize())
            .collect();
        let lengths: Vec<f32> = (0..segments)
            .map(|k| pts[k].distance(pts[(k + 1) % n]))
            .collect();
        let half = self.stroke_width * 0.5;

        // From each point to the stroke's left edge where two segments join it: `None` at an open
        // end, a reversal, or a join too sharp to miter, where each segment ends square.
        let joins: Vec<Option<Vec2>> = (0..n)
            .map(|i| {
                let incoming = match (closed, i) {
                    (true, 0) => n - 1,
                    (false, 0) => return None,
                    (false, i) if i == n - 1 => return None,
                    (_, i) => i - 1,
                };
                let (a, b) = (perp(dirs[incoming]), perp(dirs[i]));
                let miter = (a + b).try_normalize()?;
                let cos = miter.dot(b);
                (cos * MITER_LIMIT >= 1.).then(|| miter * half / cos)
            })
            .collect();

        // Each dash, or the whole line if solid, as a quad per segment it crosses. A piece that
        // reaches a corner ends at the corner's miter, so the pieces either side of it meet.
        let solid = options.dash_length == f32::MAX || options.gap_length <= 0.;
        let period = options.dash_length + options.gap_length;
        let mut s0 = 0.;
        for k in 0..segments {
            let s1 = s0 + lengths[k];
            let piece = |builder: &mut Self, t0: f32, t1: f32| {
                let side = perp(dirs[k]) * half;
                let start = if t0 <= 0. { joins[k] } else { None }.unwrap_or(side);
                let end = if t1 >= lengths[k] {
                    joins[(k + 1) % n]
                } else {
                    None
                }
                .unwrap_or(side);
                let (a, b) = (pts[k] + dirs[k] * t0, pts[k] + dirs[k] * t1);
                builder.quad(a + start, b + end, b - end, a - start);
            };
            if solid {
                piece(self, 0., lengths[k]);
            } else if options.dash_length > 0. {
                let mut dash = (s0 / period).floor();
                loop {
                    let a = dash * period;
                    if a >= s1 {
                        break;
                    }
                    let b = a + options.dash_length;
                    if b > s0 {
                        piece(self, a.max(s0) - s0, b.min(s1) - s0);
                    }
                    dash += 1.;
                }
            }
            s0 = s1;
        }
        self
    }

    /// A marker with its base at `base`, pointing along `direction` for `length`.
    fn fill_marker(&mut self, marker: StrokeMarker, base: Vec2, direction: Vec2, length: f32) {
        match marker {
            StrokeMarker::Arrowhead => {
                let side = perp(direction) * length;
                self.triangle(base + direction * length, base - side, base + side);
            }
            StrokeMarker::None => {}
        }
    }

    /// A marker's length on a segment of `segment` length: twice the stroke width, up to 40% of
    /// the segment, so a marker at each end leaves some line between them.
    fn marker_length(&self, marker: StrokeMarker, segment: f32) -> f32 {
        match marker {
            StrokeMarker::Arrowhead => (self.stroke_width * 2.0).min(segment * 0.4),
            StrokeMarker::None => 0.0,
        }
    }

    /// Writes the shapes into `mesh`, replacing what it held.
    pub fn build(self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices);
        mesh.insert_indices(Indices::U32(self.indices));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every vertex is finite and every index names one; returns the number of quads.
    fn quads(builder: &ShapeBuilder) -> usize {
        assert!(builder.vertices.iter().all(|v| v.is_finite()), "{:?}", builder.vertices);
        let count = builder.vertices.len() as u32;
        assert!(builder.indices.iter().all(|&i| i < count));
        assert_eq!(builder.indices.len() % 6, 0, "not whole quads");
        builder.indices.len() / 6
    }

    fn stroke(points: &[Vec2], options: PolygonOptions) -> ShapeBuilder {
        let mut builder = ShapeBuilder::new();
        builder.with_stroke_width(0.2).stroke_polygon(points, options);
        builder
    }

    fn closed() -> PolygonOptions {
        PolygonOptions {
            closed: true,
            ..Default::default()
        }
    }

    fn dashed(dash_length: f32, gap_length: f32) -> PolygonOptions {
        PolygonOptions {
            dash_length,
            gap_length,
            ..Default::default()
        }
    }

    const ELL: [Vec2; 3] = [Vec2::ZERO, Vec2::new(10., 0.), Vec2::new(10., 10.)];

    #[test]
    fn the_default_stroke_has_width() {
        let mut builder = ShapeBuilder::default();
        builder.stroke_polygon(&[Vec2::ZERO, Vec2::X], PolygonOptions::default());
        let ys: Vec<f32> = builder.vertices.iter().map(|v| v.y).collect();
        assert!(ys.iter().any(|&y| y > 0.4), "{ys:?}");
    }

    #[test]
    fn an_open_line_has_a_quad_per_segment() {
        assert_eq!(quads(&stroke(&ELL, PolygonOptions::default())), 2);
    }

    #[test]
    fn a_closed_square_has_four_quads_and_no_stray_vertex() {
        let square = [Vec2::ZERO, Vec2::new(4., 0.), Vec2::new(4., 4.), Vec2::new(0., 4.)];
        let mut builder = ShapeBuilder::new();
        // A shape drawn first, whose vertex 0 a stray index would reach.
        builder.fill_rect(Rect::new(100., 100., 101., 101.));
        builder.with_stroke_width(0.2).stroke_polygon(&square, closed());
        assert_eq!(quads(&builder), 5);
        let square_indices = &builder.indices[6..];
        assert!(square_indices.iter().all(|&i| i >= 4), "the square used the rect's vertices");
    }

    #[test]
    fn two_points_closed_are_a_line() {
        let builder = stroke(&[Vec2::ZERO, Vec2::new(5., 0.)], closed());
        assert_eq!(quads(&builder), 1);
    }

    #[test]
    fn repeated_points_are_dropped() {
        let points = [ELL[0], ELL[1], ELL[1], ELL[2], ELL[2]];
        assert_eq!(quads(&stroke(&points, PolygonOptions::default())), 2);
    }

    #[test]
    fn a_closing_point_equal_to_the_first_is_dropped() {
        let points = [ELL[0], ELL[1], ELL[2], ELL[0]];
        assert_eq!(quads(&stroke(&points, closed())), 3);
    }

    #[test]
    fn a_reversal_and_a_sharp_join_stay_finite_and_near() {
        let reversal = [Vec2::ZERO, Vec2::new(10., 0.), Vec2::new(5., 0.)];
        let sharp = [Vec2::ZERO, Vec2::new(10., 0.), Vec2::new(0., 0.1)];
        for points in [reversal, sharp] {
            let builder = stroke(&points, PolygonOptions::default());
            quads(&builder);
            let far = builder.vertices.iter().map(|v| v.x).fold(f32::MIN, f32::max);
            assert!(far < 10.5, "a join spiked to x = {far}");
        }
    }

    #[test]
    fn dashes_split_at_corners_and_bridge_no_gap() {
        // Dashes at 0..4, 6..10, 12..16 and 18..20 along the path; the corner is at 10.
        assert_eq!(quads(&stroke(&ELL, dashed(4., 2.))), 4);
        // Dashes at 0..6, 8..14 and 16..20: the second crosses the corner, as two quads.
        let builder = stroke(&ELL, dashed(6., 2.));
        assert_eq!(quads(&builder), 4);
        // Its two quads meet at the corner's miter: the end of the first is the start of the next.
        let corner_end = &builder.vertices[5..7];
        let corner_start = [builder.vertices[8], builder.vertices[11]];
        assert!(corner_end[0].distance(corner_start[0]) < 1e-5);
        assert!(corner_end[1].distance(corner_start[1]) < 1e-5);
    }

    #[test]
    fn an_open_line_ending_in_a_gap_does_not_wrap() {
        // Dashes at 0..4, 8..12 and 16..20, the path 20 long: no segment back to the start.
        let builder = stroke(&ELL, dashed(4., 4.));
        assert_eq!(quads(&builder), 4);
        let below = builder.vertices.iter().filter(|v| v.y > 0.2 && v.x < 9.8);
        assert_eq!(below.count(), 0, "a quad left the path");
    }

    #[test]
    fn an_arrowhead_ends_at_the_last_point() {
        let options = PolygonOptions {
            end_marker: StrokeMarker::Arrowhead,
            ..Default::default()
        };
        let mut builder = ShapeBuilder::new();
        builder
            .with_stroke_width(0.2)
            .stroke_polygon(&[Vec2::ZERO, Vec2::new(10., 0.)], options);
        let tip = builder.vertices.iter().map(|v| v.x).fold(f32::MIN, f32::max);
        assert!((tip - 10.).abs() < 1e-5, "the arrow's tip is at {tip}");
        assert_eq!(builder.indices.len(), 3 + 6);
    }

    #[test]
    fn a_circle_is_centered_on_its_center() {
        let mut builder = ShapeBuilder::new();
        builder.fill_circle(Vec2::new(5., 5.), 1., 12);
        let center = Vec3::new(5., 5., 0.);
        assert!(builder.vertices.iter().all(|v| v.distance(center) <= 1. + 1e-5));
    }

    #[test]
    fn a_wide_stroke_fills_its_rect() {
        let mut builder = ShapeBuilder::new();
        builder
            .with_stroke_width(3.)
            .stroke_rect(Rect::new(0., 0., 4., 4.));
        assert_eq!(quads(&builder), 1);
    }

    #[test]
    fn a_grid_draws_inside_lines_only() {
        let mut builder = ShapeBuilder::new();
        builder
            .with_stroke_width(0.1)
            .grid(Rect::new(0., 0., 64., 32.), 16.);
        // x = 16, 32, 48 and y = 16.
        assert_eq!(quads(&builder), 4);
    }
}
