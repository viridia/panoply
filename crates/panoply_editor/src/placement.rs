//! Placing an instance where the pointer is, and turning it as it goes down: the gesture the
//! fixture and actor tools share (D89). The instance's own shapes are cast along the pointer's ray
//! to where they first touch something, then straight down onto what is under them, as faery's
//! `PlaceFixtureTool.ts` cast them, so a mug lands on a table and a painting against a wall.
//!
//! A press fixes the place and a drag turns the instance toward the pointer, in steps, snapping to
//! quarter turns. While it does, a dial at the instance's base shows the facing, and the facing
//! shows as text beside the pointer. Escape during a drag cancels it.

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    ecs::system::SystemParam,
    picking::{
        events::PointerState,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    window::PrimaryWindow,
};
use bevy_action_map::prelude::*;
use panoply_core::{
    realm_physics::{collision_groups, group_filter},
    Realm, RealmPhysics, ToBevy, ToRapier, Viewpoint,
};
use panoply_overlays::{
    AddOverlay, DrawPlane, OverlayColor, OverlayShape, PolygonOptions, ShapeBuilder, StrokeMarker,
};
use panoply_scenery::PrecinctCache;
use rapier3d::{
    parry::query::details::ShapeCastOptions,
    prelude::{PhysicsWorld, Pose, QueryFilter, Ray, SharedShape},
};

use crate::{
    mode_selector::SceneryTool,
    scenery_pointer::{precinct_at, SceneryPointer, SceneryPointerSystems},
    terrain_cursor::level_point,
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

/// How far the pointer must move from the press, in logical pixels, before a drag turns the
/// instance.
const TURN_THRESHOLD: f32 = 5.;

/// The readout's offset from the pointer, in pixels, and its font size.
const READOUT_OFFSET: Vec2 = Vec2::new(18., 14.);
const READOUT_FONT_SIZE: f32 = 14.;

/// The gesture's state, its keys, and the dial, hand and readout that show a drag.
pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementDragState>()
            .add_overlay::<FacingDial>()
            .add_overlay::<FacingHand>()
            .add_context::<PlacementDragKeys>(|context| {
                context.active_if(|drag: Res<PlacementDragState>| drag.0.is_some());
                context
                    .bind::<CancelPlacementDrag>(KeyCode::Escape)
                    .consume();
            })
            .add_systems(Startup, spawn_drag_display)
            .add_systems(OnExit(SceneryTool::PlaceFixture), clear_drag)
            .add_systems(OnExit(SceneryTool::PlaceActor), clear_drag)
            .add_systems(
                Update,
                (show_dial, show_readout)
                    .in_set(PlacementDisplaySystems)
                    .after(SceneryPointerSystems),
            );
    }
}

/// Draws the dial and readout from the drag; the tools' updates run before it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PlacementDisplaySystems;

/// A drag in progress, held from the press to the release.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlacementDrag {
    /// The precinct the instance goes in, and its coordinates.
    pub precinct: Entity,
    pub coords: IVec2,
    /// Where the instance goes, in the world's coordinates, and its dial's radius.
    pub dial: DialPlace,
    /// In degrees, as a precinct holds it.
    pub facing: f32,
    /// Where the press was in the window, in logical pixels, and whether the pointer has moved
    /// far enough from it to turn the instance. Until it has, the instance keeps the tool's
    /// facing: the pointer's point on the instance's level can be far from the instance at the
    /// press, as on a click on a wall, whose fixture lands at the wall's foot.
    press: Option<Vec2>,
    turning: bool,
    /// Escape cancelled it: the release places nothing.
    cancelled: bool,
}

/// The drag in progress, if any.
#[derive(Resource, Default)]
pub(crate) struct PlacementDragState(Option<PlacementDrag>);

impl PlacementDragState {
    /// The drag, unless Escape cancelled it.
    fn shown(&self) -> Option<&PlacementDrag> {
        self.0.as_ref().filter(|drag| !drag.cancelled)
    }
}

/// Escape cancels a drag. Above `AppControl`, whose Escape quits, and consumed, so cancelling a
/// drag never quits.
#[derive(InputContext)]
#[context(path = "editor.placement_drag", tick = Render, priority = 500)]
pub struct PlacementDragKeys;

/// Cancel the drag: its release places nothing.
#[derive(InputAction)]
#[action(path = "editor.placement_drag.cancel", output = bool, intent = Button)]
pub struct CancelPlacementDrag;

/// The facing, as text beside the pointer, while a drag turns the instance.
#[derive(Component)]
struct FacingReadout;

fn clear_drag(mut drag: ResMut<PlacementDragState>) {
    drag.0 = None;
}

/// What the gesture did in a frame.
pub(crate) struct PlacementFrame {
    /// Where the instance would go, in the world's coordinates, and its facing in degrees: from
    /// the drag, or where a click would place it. `None` while the pointer is over nothing, or
    /// after Escape, until the release.
    pub target: Option<(Vec3, f32)>,
    /// The drag a release ended, unless Escape cancelled it: the placement to commit.
    pub released: Option<PlacementDrag>,
}

/// The pointer, the keys and the drag, for a placing tool's update.
#[derive(SystemParam)]
pub(crate) struct PlacementGesture<'w, 's> {
    pointer: Res<'w, SceneryPointer>,
    pointers: Res<'w, PointerState>,
    keys: ContextActions<'w, 's, PlacementDragKeys>,
    drag: ResMut<'w, PlacementDragState>,
    was_pressed: Local<'s, bool>,
    ui_nodes: Query<'w, 's, (), With<Node>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    viewpoint: Res<'w, Viewpoint>,
    realms: Query<'w, 's, &'static RealmPhysics>,
    cache: Res<'w, PrecinctCache>,
}

impl PlacementGesture<'_, '_> {
    /// Whether a drag is turning an instance, showing the dial.
    pub fn dragging(&self) -> bool {
        self.drag.shown().is_some()
    }

    /// Follows the pointer with an instance of `shapes`, facing `facing` degrees while no drag
    /// turns it; starts, turns, cancels and ends drags.
    pub fn update(&mut self, shapes: &[(SharedShape, Pose)], facing: f32) -> PlacementFrame {
        // A press in the view, not on the panel or a widget over the view.
        let pressed = self
            .pointers
            .get(PointerId::Mouse, PointerButton::Primary)
            .is_some_and(|state| state.pressing.keys().any(|e| !self.ui_nodes.contains(*e)));
        let just_pressed = pressed && !*self.was_pressed;
        let just_released = !pressed && *self.was_pressed;
        *self.was_pressed = pressed;

        if self.keys.fired::<CancelPlacementDrag>() {
            if let Some(drag) = self.drag.0.as_mut() {
                drag.cancelled = true;
            }
        }

        let realm = self.viewpoint.realm;
        let world = realm
            .and_then(|realm| self.realms.get(realm).ok())
            .map(RealmPhysics::world);
        let (ray, point) = (self.pointer.ray, self.pointer.point);
        let land = |facing: f32| {
            world
                .zip(ray)
                .and_then(|(world, ray)| cast_placement(world, shapes, facing.to_radians(), ray))
                .or(point)
        };

        let cursor = self.windows.single().ok().and_then(Window::cursor_position);
        if just_pressed {
            self.drag.0 = land(facing).and_then(|position| {
                let (precinct, coords) = precinct_at(&self.cache, realm?, position)?;
                Some(PlacementDrag {
                    precinct,
                    coords,
                    dial: DialPlace {
                        center: position,
                        radius: dial_radius(shapes),
                    },
                    facing,
                    press: cursor,
                    turning: false,
                    cancelled: false,
                })
            });
        } else if let Some(drag) = self.drag.0.as_mut() {
            drag.turning |= cursor
                .zip(drag.press)
                .is_some_and(|(cursor, press)| cursor.distance(press) >= TURN_THRESHOLD);
            let dial = drag.dial;
            let point = ray
                .filter(|_| drag.turning)
                .and_then(|ray| level_point(ray, dial.center.y));
            if let Some(facing) =
                point.and_then(|point| facing_toward(dial.center.xz(), point.xz(), dial.radius))
            {
                drag.facing = facing;
            }
        }

        let released = just_released
            .then(|| self.drag.0.take())
            .flatten()
            .filter(|drag| !drag.cancelled);
        let target = match self.drag.0 {
            Some(drag) => (!drag.cancelled).then_some((drag.dial.center, drag.facing)),
            None => land(facing).map(|position| (position, facing)),
        };
        PlacementFrame { target, released }
    }
}

/// `facing`, in degrees, turned by `turns` quarter turns, right for positive, as the wall tool
/// turns. A facing between quarter turns takes the next one that way first, so the keys always
/// return to them.
pub(crate) fn turn_facing(facing: f32, turns: i8) -> f32 {
    let quarters = facing / 90.;
    let next = if turns > 0 {
        quarters.ceil() - turns as f32
    } else {
        quarters.floor() - turns as f32
    };
    (next * 90.).rem_euclid(360.) + 0.
}

/// The drag's keys, and the dial, its hand and the readout, hidden until a drag shows them.
fn spawn_drag_display(asset_server: Res<AssetServer>, mut commands: Commands) {
    // Without it, `ContextActions` finds no context, and the tools' updates are skipped.
    commands.spawn((Name::new("PlacementDragKeys"), PlacementDragKeys));
    let overlay = || {
        (
            OverlayColor {
                base: css::SILVER.with_alpha(0.9).into(),
                underlay: 0.6,
            },
            Propagate(RenderLayers::none()),
            Transform::default(),
            Visibility::default(),
        )
    };
    commands.spawn((Name::new("FacingDial"), FacingDial::default(), overlay()));
    commands.spawn((Name::new("FacingHand"), FacingHand::default(), overlay()));
    commands.spawn((
        Name::new("FacingReadout"),
        FacingReadout,
        Text::default(),
        TextFont {
            font: FontSource::Handle(asset_server.load("fonts/Rubik/Rubik-VariableFont_wght.ttf")),
            font_size: FontSize::Px(READOUT_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

/// Shows the dial and its hand while a drag turns an instance, in the viewed realm.
fn show_dial(
    drag: Res<PlacementDragState>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    mut dials: Query<(&mut FacingDial, &mut Propagate<RenderLayers>), Without<FacingHand>>,
    mut hands: Query<(
        &mut FacingHand,
        &mut OverlayColor,
        &mut Propagate<RenderLayers>,
    )>,
) {
    let layer = viewpoint
        .realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    let shown = drag.shown();
    for (mut dial, mut propagate) in &mut dials {
        dial.set_if_neq(FacingDial(shown.map(|drag| drag.dial)));
        propagate.set_if_neq(Propagate(layer.clone()));
    }
    let snapped = shown.is_some_and(|drag| is_quarter_turn(drag.facing));
    for (mut hand, mut color, mut propagate) in &mut hands {
        hand.set_if_neq(FacingHand(shown.map(|drag| (drag.dial, drag.facing))));
        let base = if snapped { css::GOLD } else { css::SILVER };
        color.base = base.with_alpha(0.9).into();
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}

/// Shows the facing beside the pointer while a drag turns an instance.
fn show_readout(
    drag: Res<PlacementDragState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut readouts: Query<(&mut Text, &mut Node), With<FacingReadout>>,
) {
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    for (mut text, mut node) in &mut readouts {
        let Some((cursor, drag)) = cursor.zip(drag.shown()) else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        let place = cursor + READOUT_OFFSET;
        if node.display != Display::Flex || node.left != px(place.x) || node.top != px(place.y) {
            node.display = Display::Flex;
            node.left = px(place.x);
            node.top = px(place.y);
        }
        let shown = format!("{}°", drag.facing.round());
        if text.0 != shown {
            text.0 = shown;
        }
    }
}

/// What the cast can land on: everything but the canopy, which the pointer passes through (D39),
/// and markers, which nothing stands on.
fn cast_filter() -> QueryFilter<'static> {
    group_filter(collision_groups::BLOCKING.difference(collision_groups::CANOPY))
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
    let (along, normal) = first_hit(
        world,
        shapes,
        rotation,
        ray.origin,
        *ray.direction,
        CAST_RANGE,
    )?;
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
    pub(crate) fn builder(self, builder: &mut ShapeBuilder) -> &mut ShapeBuilder {
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(self.center.y + DIAL_LIFT)
            .with_stroke_width(self.stroke())
    }

    /// The point at `radius` from the center toward `degrees`, in the world's x and z.
    pub(crate) fn toward(self, degrees: f32, radius: f32) -> Vec2 {
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
        let on_table =
            cast_placement(world, &mug, 0., ray_toward(Vec3::new(2.5, 0.8, 0.))).unwrap();
        assert!(
            (on_table.y - 0.8).abs() < 0.01,
            "on the table at {on_table}"
        );

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
        assert!(
            (hung.x - -0.95).abs() < 0.01,
            "flush with the wall at {hung}"
        );
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
    fn keys_turn_by_quarter_turns_as_the_wall_tool_turns() {
        assert_eq!(turn_facing(0., 1), 270.);
        assert_eq!(turn_facing(270., -2), 90.);
    }

    #[test]
    fn keys_return_a_facing_between_quarter_turns_to_one() {
        assert_eq!(turn_facing(35., 1), 0.);
        assert_eq!(turn_facing(35., -1), 90.);
        assert_eq!(turn_facing(305., -1), 0.);
        assert_eq!(turn_facing(35., 2), 270.);
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
