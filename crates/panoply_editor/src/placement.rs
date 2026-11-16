//! Placing an instance where the pointer is, and turning it as it goes down: what the fixture tool
//! does, and the actor tool will. The instance's own shapes are cast along the pointer's ray to
//! where they first touch something, then straight down onto what is under them, as faery's
//! `PlaceFixtureTool.ts` cast them, so a mug lands on a table and a painting against a wall.
//!
//! A press fixes the place and a drag turns the instance toward the pointer, in steps, snapping to
//! quarter turns. While it does, a dial at the instance's base shows the facing.

use bevy::prelude::*;
use panoply_core::{realm_physics::collision_groups, ToBevy, ToRapier};
use panoply_overlays::{DrawPlane, OverlayShape, PolygonOptions, ShapeBuilder, StrokeMarker};
use rapier3d::{
    parry::query::details::ShapeCastOptions,
    prelude::{
        Group, InteractionGroups, InteractionTestMode, PhysicsWorld, Pose, QueryFilter, Ray,
        SharedShape,
    },
};

/// How far along the pointer's ray an instance is cast, in meters.
const CAST_RANGE: f32 = 1000.;

/// How far an instance drops onto what is under it, in meters, as faery's did.
const DROP_RANGE: f32 = 2.;

/// How far short of what it touches an instance stops, so it rests on it, not in it.
const CAST_GAP: f32 = 0.001;

/// How far an instance backs off what the pointer's cast touched, along its surface's normal,
/// before it drops: dropped past a wall a millimeter away, parry finds it touching the wall.
const SURFACE_GAP: f32 = 0.005;

/// The steps a drag turns an instance by, in degrees.
const FACING_STEP: f32 = 5.;

/// How near a quarter turn a drag must come, in degrees, to snap to it.
const FACING_SNAP: f32 = 10.;

/// The dial's smallest radius, in meters, so a small instance's is still readable.
const DIAL_MIN_RADIUS: f32 = 0.5;

/// The dial's radius, as a multiple of the instance's footprint, so the ring clears it.
const DIAL_MARGIN: f32 = 1.25;

/// The dial's middle, where the pointer does not turn the instance, as a fraction of its radius.
const DEAD_ZONE: f32 = 0.35;

/// The dial's ticks, in degrees apart, and the fraction of its radius each kind reaches in.
const TICK_SPACING: u32 = 15;
const TICK_LENGTH: f32 = 0.1;
const QUARTER_TICK_LENGTH: f32 = 0.25;

/// How far above the instance's base the dial is drawn, so its depth does not tie with a floor's.
const DIAL_LIFT: f32 = 0.02;

/// What the cast can land on: everything but the canopy, which the pointer passes through (D39).
fn cast_filter() -> QueryFilter<'static> {
    QueryFilter::default().groups(InteractionGroups::new(
        Group::ALL,
        !collision_groups::CANOPY,
        InteractionTestMode::And,
    ))
}

/// How far `shapes`, at `origin` and turned by `rotation`, move along `direction` before the first
/// of them, or the origin itself, touches something, within `range`, and the normal of the surface
/// touched. The origin is cast as well so that it never goes into the ground, as faery's zero-sized
/// box kept it out, and so that an instance with no shapes casts as a point.
fn first_hit(
    world: &PhysicsWorld,
    shapes: &[(SharedShape, Pose)],
    rotation: Quat,
    origin: Vec3,
    direction: Vec3,
    range: f32,
) -> Option<(f32, Vec3)> {
    let start = Pose::from_parts(origin.to_rapier(), rotation.to_rapier());
    let options = ShapeCastOptions {
        max_time_of_impact: range,
        target_distance: 0.,
        stop_at_penetration: true,
        compute_impact_geometry_on_penetration: false,
    };
    let ray = Ray::new(origin.to_rapier(), direction.to_rapier());
    let point = world
        .cast_ray_and_get_normal(&ray, range, true, cast_filter())
        .map(|(_, hit)| (hit.time_of_impact, hit.normal.to_bevy()));
    shapes
        .iter()
        .filter_map(|(shape, pose)| {
            world.cast_shape(
                &(start * *pose),
                direction.to_rapier(),
                &**shape,
                options,
                cast_filter(),
            )
        })
        .map(|(_, hit)| (hit.time_of_impact, hit.normal1.to_bevy()))
        .chain(point)
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// Where an instance of `shapes`, facing `facing` radians, lands when cast along `ray`: where it
/// first touches something, then dropped onto what is under it. `None` if the ray meets nothing.
pub(crate) fn cast_placement(
    world: &PhysicsWorld,
    shapes: &[(SharedShape, Pose)],
    facing: f32,
    ray: Ray3d,
) -> Option<Vec3> {
    let rotation = Quat::from_rotation_y(facing);
    let (along, normal) =
        first_hit(world, shapes, rotation, ray.origin, *ray.direction, CAST_RANGE)?;
    let touch = ray.get_point((along - CAST_GAP).max(0.)) + normal * SURFACE_GAP;
    let drop = first_hit(world, shapes, rotation, touch, Vec3::NEG_Y, DROP_RANGE)
        .map_or(0., |(drop, _)| (drop - CAST_GAP).max(0.));
    Some(touch - Vec3::Y * drop)
}

/// The radius of the dial around an instance of `shapes`: clear of its footprint, turned any way.
pub(crate) fn dial_radius(shapes: &[(SharedShape, Pose)]) -> f32 {
    let footprint = shapes
        .iter()
        .map(|(shape, pose)| {
            let aabb = shape.compute_aabb(pose);
            let (min, max) = (aabb.mins.to_bevy().xz(), aabb.maxs.to_bevy().xz());
            min.abs().max(max.abs()).length()
        })
        .fold(0., f32::max);
    (footprint * DIAL_MARGIN).max(DIAL_MIN_RADIUS)
}

/// The facing, in degrees as a precinct holds it, that turns an instance at `center` toward
/// `point`, both in the world's x and z; `None` while `point` is in the middle of a dial of
/// `radius`, too near to say.
pub(crate) fn facing_toward(center: Vec2, point: Vec2, radius: f32) -> Option<f32> {
    let offset = point - center;
    if offset.length() < radius * DEAD_ZONE {
        return None;
    }
    // `Quat::from_rotation_y(a)` turns +z, an instance's front, to (sin a, cos a) in x and z.
    Some(quantize_facing(offset.x.atan2(offset.y).to_degrees()))
}

/// `degrees` in steps, snapped to a quarter turn near one, from 0 to 360.
fn quantize_facing(degrees: f32) -> f32 {
    let quarter = (degrees / 90.).round() * 90.;
    let facing = if (degrees - quarter).abs() <= FACING_SNAP {
        quarter
    } else {
        (degrees / FACING_STEP).round() * FACING_STEP
    };
    // Adding zero turns -0 into 0.
    facing.rem_euclid(360.) + 0.
}

/// Whether `degrees` is a quarter turn.
pub(crate) fn is_quarter_turn(degrees: f32) -> bool {
    degrees % 90. == 0.
}

/// Where a dial is drawn: the instance's base, in the world's coordinates, and the dial's radius.
#[derive(Reflect, Clone, Copy, PartialEq, Debug)]
pub struct DialPlace {
    pub center: Vec3,
    pub radius: f32,
}

impl DialPlace {
    /// The dial's strokes, in meters: wider for a larger dial, which is seen from farther away.
    fn stroke(self) -> f32 {
        (self.radius * 0.06).max(0.03)
    }

    /// A builder drawing on the ground at the dial's height.
    fn builder(self, builder: &mut ShapeBuilder) -> &mut ShapeBuilder {
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(self.center.y + DIAL_LIFT)
            .with_stroke_width(self.stroke())
    }

    /// The point at `radius` from the center toward `degrees`, in the world's x and z.
    fn toward(self, degrees: f32, radius: f32) -> Vec2 {
        let (sin, cos) = degrees.to_radians().sin_cos();
        self.center.xz() + Vec2::new(sin, cos) * radius
    }
}

/// The dial a drag turns an instance on: a ring, ticks every 15° and longer ones at the quarter
/// turns, and a ring around the middle, where the pointer does not turn it. Hidden while `None`.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct FacingDial(pub Option<DialPlace>);

impl OverlayShape for FacingDial {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let Some(dial) = self.0 else {
            return;
        };
        let builder = dial.builder(builder);
        let center = dial.center.xz();
        builder.stroke_circle(center, dial.radius, 64);
        builder.stroke_circle(center, dial.radius * DEAD_ZONE, 32);
        for degrees in (0..360).step_by(TICK_SPACING as usize) {
            let degrees = degrees as f32;
            let length = if is_quarter_turn(degrees) {
                QUARTER_TICK_LENGTH
            } else {
                TICK_LENGTH
            };
            builder.stroke_polygon(
                &[
                    dial.toward(degrees, dial.radius * (1. - length)),
                    dial.toward(degrees, dial.radius - dial.stroke() * 0.5),
                ],
                PolygonOptions::default(),
            );
        }
    }
}

/// The dial's hand: an arrow from the middle out past the ring, along the facing, in degrees as a
/// precinct holds it. Hidden while `None`.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct FacingHand(pub Option<(DialPlace, f32)>);

impl OverlayShape for FacingHand {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let Some((dial, degrees)) = self.0 else {
            return;
        };
        dial.builder(builder).stroke_polygon(
            &[
                dial.toward(degrees, dial.radius * DEAD_ZONE),
                dial.toward(degrees, dial.radius * 1.2),
            ],
            PolygonOptions {
                end_marker: StrokeMarker::Arrowhead,
                ..default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use panoply_core::RealmPhysics;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    use super::*;

    /// A floor at y = 0, a table 0.8 high from x = 2 to 3, and a wall along x = -1.
    fn room() -> RealmPhysics {
        let mut physics = RealmPhysics::default();
        let body = physics.insert_body(RigidBodyBuilder::fixed().build());
        for (half, center) in [
            (Vec3::new(10., 0.1, 10.), Vec3::new(0., -0.1, 0.)),
            (Vec3::new(0.5, 0.4, 0.5), Vec3::new(2.5, 0.4, 0.)),
            (Vec3::new(0.1, 2., 10.), Vec3::new(-1.1, 2., 0.)),
        ] {
            let collider = ColliderBuilder::cuboid(half.x, half.y, half.z)
                .translation(center.to_rapier())
                .build();
            physics.insert_collider(body, collider);
        }
        // The query pipeline takes in new colliders when the world steps.
        physics.step(1. / 64.);
        physics
    }

    /// A ray from above and in front, toward `point`.
    fn ray_toward(point: Vec3) -> Ray3d {
        let origin = point + Vec3::new(0., 6., 8.);
        Ray3d::new(origin, Dir3::new(point - origin).unwrap())
    }

    #[test]
    fn an_instance_lands_on_the_floor_on_a_table_and_against_a_wall() {
        let physics = room();
        let world = physics.world();
        let mug = [(
            SharedShape::cuboid(0.05, 0.05, 0.05),
            Pose::from_translation(Vec3::new(0., 0.05, 0.).to_rapier()),
        )];
        let on_floor = cast_placement(world, &mug, 0., ray_toward(Vec3::new(0., 0., 0.))).unwrap();
        assert!(on_floor.y.abs() < 0.01, "on the floor at {on_floor}");
        let on_table = cast_placement(world, &mug, 0., ray_toward(Vec3::new(2.5, 0.8, 0.))).unwrap();
        assert!((on_table.y - 0.8).abs() < 0.01, "on the table at {on_table}");

        // A painting, raised in its exemplar, touches the wall and drops to the floor.
        let painting = [(
            SharedShape::cuboid(0.4, 0.5, 0.05),
            Pose::from_translation(Vec3::new(0., 1.2, 0.).to_rapier()),
        )];
        let facing_out = 90f32.to_radians();
        let origin = Vec3::new(3., 1.5, 0.);
        let ray = Ray3d::new(origin, Dir3::new(Vec3::new(-1., -0.1, 0.)).unwrap());
        let hung = cast_placement(world, &painting, facing_out, ray).unwrap();
        assert!(hung.y.abs() < 0.01, "dropped to the floor at {hung}");
        assert!((hung.x - -0.95).abs() < 0.01, "flush with the wall at {hung}");
    }

    #[test]
    fn a_drag_turns_in_steps_and_snaps_to_quarter_turns() {
        assert_eq!(quantize_facing(33.), 35.);
        assert_eq!(quantize_facing(-33.), 325.);
        // Within 10° of a quarter turn, the quarter turn.
        assert_eq!(quantize_facing(81.), 90.);
        assert_eq!(quantize_facing(-8.), 0.);
        assert_eq!(quantize_facing(355.), 0.);
        assert_eq!(quantize_facing(-180.), 180.);
        // Just outside it, the nearest step.
        assert_eq!(quantize_facing(78.), 80.);
    }

    #[test]
    fn the_facing_turns_the_front_toward_the_pointer() {
        let center = Vec2::new(10., 10.);
        for (point, degrees) in [
            (Vec2::new(10., 12.), 0.),
            (Vec2::new(12., 10.), 90.),
            (Vec2::new(10., 8.), 180.),
            (Vec2::new(8., 10.), 270.),
            (Vec2::new(12., 12.), 45.),
        ] {
            let facing = facing_toward(center, point, 1.).unwrap();
            assert_eq!(facing, degrees, "toward {point}");
            // As a precinct turns it: its front, +z, points at the pointer.
            let front = Quat::from_rotation_y(facing.to_radians()) * Vec3::Z;
            let toward = (point - center).normalize();
            assert!(front.xz().distance(toward) < 1e-5, "{front} at {facing}°");
        }
    }

    #[test]
    fn the_middle_of_the_dial_does_not_turn() {
        let center = Vec2::ZERO;
        assert_eq!(facing_toward(center, Vec2::new(0.3, 0.), 1.), None);
        assert_eq!(facing_toward(center, Vec2::new(0.4, 0.), 1.), Some(90.));
    }

    #[test]
    fn the_dial_clears_the_footprint() {
        assert_eq!(dial_radius(&[]), DIAL_MIN_RADIUS);
        let bed = [(
            SharedShape::cuboid(1., 0.3, 0.5),
            Pose::from_translation(Vec3::new(0., 0.3, 0.).to_rapier()),
        )];
        let corner = Vec2::new(1., 0.5).length();
        assert!((dial_radius(&bed) - corner * DIAL_MARGIN).abs() < 1e-5);
    }
}
