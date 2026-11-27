//! The gizmo on the instance selected in scenery mode's selection mode: arrows that move it along
//! the world's x, y and z, and a ring that turns it about y. Instances turn only about y, as a
//! precinct holds their facing; a tilt is built into the exemplar. The handles all show at once,
//! so the gizmo has no modes (D126).
//!
//! A drag shows the instance where it would go and commits at the release, as one edit; Escape
//! cancels it. Moves snap each coordinate to a quarter meter, and turns snap as a placement's do;
//! Shift held moves and turns freely. A move past the precinct's edge takes the instance into the
//! precinct there. Delete removes the selected instance, and End or F drops a fixture onto what is
//! under it, as Unreal's editor drops to the floor on End. An actor, which falls in the editor
//! too, is moved to where it lands, so a lift onto an upper floor leaves it standing there; a move
//! with nothing under it to stand on is refused.
//!
//! With a path's point chosen (`path_edit`), the gizmo is on the point, its arrows alone, since a
//! point has no facing, and moves it within its path; Delete takes the point out and End drops it,
//! and Escape goes back to the whole path.
//!
//! The gizmo keeps its size on screen, and is drawn as overlays, which show through the scene,
//! fainter, where it hides them. A picking backend of its own reports its handles ahead of the
//! scenery under them, so a press on one selects nothing.

use std::borrow::Cow;

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    ecs::system::SystemParam,
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        events::PointerState,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
        PickingSystems,
    },
    prelude::*,
};
use bevy_action_map::prelude::*;
use panoply_actors::{
    capsule, ActorHeld, ActorIntent, ActorMotion, ActorScale, CollisionSize, PlacedAs,
};
use panoply_core::{
    realm_physics::owner_shown, PrimaryCamera, Realm, RealmPhysics, ToRapier, Viewpoint,
};
use panoply_overlays::{
    AddOverlay, DrawPlane, OverlayColor, OverlayShape, PolygonOptions, ShapeBuilder, StrokeMarker,
};
use panoply_precinct::{PrecinctAsset, SceneryInstanceId};
use panoply_scenery::{
    precinct::Precinct, scenery_element::SceneryElement, ColliderDesc, MarkerPath, PrecinctCache,
    SceneryColliders, PRECINCT_SIZE_F,
};
use rapier3d::prelude::{Pose, SharedShape};

use crate::{
    instance_selection::{InstancePart, Reselect, ScenerySelection},
    marker_overlays::PathPreview,
    mode_selector::SceneryTool,
    path_edit::{set_path_points, surface_point, without_point},
    placement::{landing, quantize_facing, TurnReadout},
    precinct_edit::PrecinctEdits,
    scenery_pointer::{precinct_at, SceneryPointer, SceneryPointerSystems},
    terrain_cursor::level_point,
    toast::ShowToast,
};

/// The gizmo's size, in meters per meter from the camera, so it keeps its size on screen. Its
/// shapes are drawn in units of it.
const GIZMO_SCALE: f32 = 0.1;

/// Where the arrows start and end, from the instance's origin. The arrowhead ends at the end.
const ARROW_START: f32 = 0.2;
const ARROW_END: f32 = 1.0;

/// The ring's radius, and its ticks: degrees apart, and the length of each kind.
const RING_RADIUS: f32 = 1.25;
const TICK_SPACING: usize = 15;
const TICK_LENGTH: f32 = 0.08;
const QUARTER_TICK_LENGTH: f32 = 0.18;

/// The points around the ring, for drawing it and for hitting it.
const RING_SEGMENTS: u32 = 64;

/// The strokes' width: the arrows', and the ring's, which is thinner, so its ticks read.
const ARROW_STROKE: f32 = 0.035;
const RING_STROKE: f32 = 0.025;

/// How near a handle's middle line the pointer must be to take it, in logical pixels: well beyond
/// the strokes, so a thin handle is easy to take.
pub(crate) const HIT_PIXELS: f32 = 14.;

/// Above the camera's own hits, which scenery's and the ground's backends report at its order,
/// and below its UI's, which Bevy reports half an order above.
pub(crate) const GIZMO_ORDER: f32 = 0.25;

/// The step a move snaps each coordinate to, in meters.
const MOVE_STEP: f32 = 0.25;

/// A free turn rounds to a tenth of a degree.
const FREE_TURN_DIVISIONS: f32 = 10.;

/// How far End lets an instance fall, and a moved actor, in meters.
const DROP_FALL: f32 = 50.;

/// How far above a path's point End starts looking down for what is under it, so a point just
/// under the ground, as an import leaves some, comes up onto it.
const POINT_DROP_LIFT: f32 = 0.1;

/// The opacity of the handles a drag is not using.
const IDLE_ALPHA: f32 = 0.3;

pub struct InstanceGizmoPlugin;

impl Plugin for InstanceGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GizmoPlace>()
            .init_resource::<GizmoDragState>()
            .init_resource::<GizmoActive>()
            .register_type::<GizmoActive>()
            .register_type::<GizmoHandle>()
            .add_overlay::<GizmoHandle>()
            .add_context::<InstanceGizmoKeys>(|context| {
                context.active_in_state(SceneryTool::SelectInstance);
                context.bind::<DeleteInstance>(KeyCode::Delete);
                context.bind::<DeleteInstance>(KeyCode::Backspace);
                // F for floor, on keyboards with no End.
                context.bind::<DropInstance>(KeyCode::End);
                context.bind::<DropInstance>(KeyCode::KeyF);
                context.bind::<FreeMove>(KeyCode::ShiftLeft);
                context.bind::<FreeMove>(KeyCode::ShiftRight);
                context.bind::<ExtendPath>(KeyCode::ControlLeft);
                context.bind::<ExtendPath>(KeyCode::ControlRight);
            })
            .add_context::<GizmoDragKeys>(|context| {
                context.active_if(|drag: Res<GizmoDragState>| drag.0.is_some());
                context.bind::<CancelGizmoDrag>(KeyCode::Escape).consume();
            })
            .add_context::<ChosenPartKeys>(|context| {
                context.active_if(
                    |drag: Res<GizmoDragState>,
                     selection: Res<ScenerySelection>,
                     tool: Option<Res<State<SceneryTool>>>| {
                        drag.0.is_none()
                            && selection.part.is_some()
                            && tool.is_some_and(|tool| *tool.get() == SceneryTool::SelectInstance)
                    },
                );
                context.bind::<ClearPart>(KeyCode::Escape).consume();
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("InstanceGizmoKeys"), InstanceGizmoKeys));
                commands.spawn((Name::new("GizmoDragKeys"), GizmoDragKeys));
                commands.spawn((Name::new("ChosenPartKeys"), ChosenPartKeys));
            })
            .add_systems(OnEnter(SceneryTool::SelectInstance), spawn_gizmo)
            .add_systems(OnExit(SceneryTool::SelectInstance), end_drag)
            .add_systems(
                PreUpdate,
                pick_gizmo
                    .in_set(PickingSystems::Backend)
                    .run_if(in_state(SceneryTool::SelectInstance)),
            )
            .add_systems(
                Update,
                (
                    drag_gizmo,
                    drop_selected,
                    delete_selected,
                    clear_part,
                    show_gizmo,
                )
                    .chain()
                    .after(SceneryPointerSystems)
                    .run_if(in_state(SceneryTool::SelectInstance)),
            );
    }
}

/// The gizmo's keys while selecting.
#[derive(InputContext)]
#[context(path = "editor.instance_gizmo", tick = Render)]
pub struct InstanceGizmoKeys;

/// Delete the selected instance.
#[derive(InputAction)]
#[action(path = "editor.instance_gizmo.delete", output = bool, intent = Button)]
pub struct DeleteInstance;

/// Drop the selected fixture onto what is under it.
#[derive(InputAction)]
#[action(path = "editor.instance_gizmo.drop", output = bool, intent = Button)]
pub struct DropInstance;

/// Held, moves and turns without snapping.
#[derive(InputAction)]
#[action(path = "editor.instance_gizmo.free", output = bool, intent = Button)]
pub struct FreeMove;

/// Escape cancels a drag. Above `AppControl`, whose Escape quits, and consumed, as the placement
/// drag's is.
#[derive(InputContext)]
#[context(path = "editor.instance_gizmo_drag", tick = Render, priority = 500)]
pub struct GizmoDragKeys;

/// Held, a click adds a point to the selected path (`path_edit`).
#[derive(InputAction)]
#[action(path = "editor.instance_gizmo.extend_path", output = bool, intent = Button)]
pub struct ExtendPath;

/// Cancel the drag: the instance goes back, and nothing is edited.
#[derive(InputAction)]
#[action(path = "editor.instance_gizmo_drag.cancel", output = bool, intent = Button)]
pub struct CancelGizmoDrag;

/// Escape, while a part of the selected instance is chosen and nothing is dragged: above
/// `AppControl`, whose Escape quits, and consumed, as the drag's is.
#[derive(InputContext)]
#[context(path = "editor.chosen_part", tick = Render, priority = 500)]
pub struct ChosenPartKeys;

/// Choose no part: the gizmo goes back to the whole instance.
#[derive(InputAction)]
#[action(path = "editor.chosen_part.clear", output = bool, intent = Button)]
pub struct ClearPart;

/// The handle under the pointer, and the one a drag holds.
#[derive(Resource, Reflect, Clone, Copy, PartialEq, Debug, Default)]
#[reflect(Resource)]
pub struct GizmoActive {
    pub hovered: Option<GizmoPart>,
    pub held: Option<GizmoPart>,
}

/// A handle of the gizmo.
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GizmoPart {
    #[default]
    AxisX,
    AxisY,
    AxisZ,
    /// Turns the instance about y.
    Ring,
}

const PARTS: [GizmoPart; 4] = [
    GizmoPart::AxisX,
    GizmoPart::AxisY,
    GizmoPart::AxisZ,
    GizmoPart::Ring,
];

impl GizmoPart {
    /// The way an arrow points, in the world.
    fn axis(self) -> Option<Vec3> {
        match self {
            GizmoPart::AxisX => Some(Vec3::X),
            GizmoPart::AxisY => Some(Vec3::Y),
            GizmoPart::AxisZ => Some(Vec3::Z),
            GizmoPart::Ring => None,
        }
    }

    fn color(self) -> Srgba {
        match self {
            GizmoPart::AxisX => Srgba::rgb(0.93, 0.27, 0.33),
            GizmoPart::AxisY => Srgba::rgb(0.40, 0.85, 0.35),
            GizmoPart::AxisZ => Srgba::rgb(0.30, 0.55, 1.0),
            GizmoPart::Ring => css::SILVER,
        }
    }
}

/// A handle's shape, in the gizmo's units, at its entity's origin. An arrow is drawn up its
/// entity's y, in its xy plane, which its entity turns to the arrow's axis, facing the camera.
/// Hidden while not `shown`.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct GizmoHandle {
    pub part: GizmoPart,
    pub shown: bool,
    /// The ring's mark: the instance's facing, in degrees.
    pub facing: f32,
}

impl OverlayShape for GizmoHandle {
    fn draw(&self, builder: &mut ShapeBuilder) {
        if !self.shown {
            return;
        }
        let arrow = PolygonOptions {
            end_marker: StrokeMarker::Arrowhead,
            ..default()
        };
        if self.part != GizmoPart::Ring {
            builder
                .with_stroke_width(ARROW_STROKE)
                .with_draw_plane(DrawPlane::XY)
                .stroke_polygon(
                    &[Vec2::new(0., ARROW_START), Vec2::new(0., ARROW_END)],
                    arrow,
                );
            return;
        }
        builder
            .with_stroke_width(RING_STROKE)
            .with_draw_plane(DrawPlane::XZ)
            .stroke_circle(Vec2::ZERO, RING_RADIUS, RING_SEGMENTS);
        for degrees in (0..360).step_by(TICK_SPACING) {
            let degrees = degrees as f32;
            let length = if degrees % 90. == 0. {
                QUARTER_TICK_LENGTH
            } else {
                TICK_LENGTH
            };
            builder.stroke_polygon(
                &[
                    toward(degrees, RING_RADIUS - length),
                    toward(degrees, RING_RADIUS),
                ],
                PolygonOptions::default(),
            );
        }
        builder.stroke_polygon(
            &[
                toward(self.facing, RING_RADIUS * 0.8),
                toward(self.facing, RING_RADIUS * 1.2),
            ],
            arrow,
        );
    }
}

/// The point `radius` from the middle toward `degrees`, a facing, in the world's x and z.
fn toward(degrees: f32, radius: f32) -> Vec2 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Vec2::new(sin, cos) * radius
}

/// Where the gizmo is, as last shown, for the picking backend: hidden while `None`.
#[derive(Resource, Default)]
struct GizmoPlace(Option<GizmoFrame>);

/// The gizmo's place and size.
#[derive(Clone, Copy, PartialEq, Debug)]
struct GizmoFrame {
    /// The instance's origin, in the world.
    origin: Vec3,
    /// Meters per gizmo unit.
    scale: f32,
    /// Whether it has its ring: a path's point does not turn.
    turns: bool,
}

impl GizmoFrame {
    fn new(origin: Vec3, camera: Vec3, turns: bool) -> Self {
        Self {
            origin,
            scale: (camera - origin).length() * GIZMO_SCALE,
            turns,
        }
    }

    /// An arrow's ends, in the world.
    fn arrow(self, part: GizmoPart) -> Option<(Vec3, Vec3)> {
        let axis = part.axis()?;
        Some((
            self.origin + axis * ARROW_START * self.scale,
            self.origin + axis * ARROW_END * self.scale,
        ))
    }

    /// A point in the gizmo's x and z, in the world.
    fn on_level(self, point: Vec2) -> Vec3 {
        self.origin + Vec3::new(point.x, 0., point.y) * self.scale
    }
}

/// The handle of `frame` nearest `cursor`, within reach, where `project` puts points of the world
/// on the screen.
fn part_at(
    frame: GizmoFrame,
    cursor: Vec2,
    project: impl Fn(Vec3) -> Option<Vec2>,
) -> Option<GizmoPart> {
    let mut nearest: Option<(f32, GizmoPart)> = None;
    let mut consider = |distance: f32, part: GizmoPart| {
        if distance <= HIT_PIXELS && nearest.is_none_or(|(best, _)| distance < best) {
            nearest = Some((distance, part));
        }
    };
    for part in [GizmoPart::AxisX, GizmoPart::AxisY, GizmoPart::AxisZ] {
        let Some((a, b)) = frame.arrow(part) else {
            continue;
        };
        if let (Some(a), Some(b)) = (project(a), project(b)) {
            consider(segment_distance(cursor, a, b), part);
        }
    }
    if !frame.turns {
        return nearest.map(|(_, part)| part);
    }
    let ring: Option<Vec<Vec2>> = (0..=RING_SEGMENTS)
        .map(|i| {
            let degrees = i as f32 * 360. / RING_SEGMENTS as f32;
            project(frame.on_level(toward(degrees, RING_RADIUS)))
        })
        .collect();
    if let Some(ring) = ring {
        let distance = ring
            .windows(2)
            .map(|pair| segment_distance(cursor, pair[0], pair[1]))
            .fold(f32::MAX, f32::min);
        consider(distance, GizmoPart::Ring);
    }
    nearest.map(|(_, part)| part)
}

/// The distance from `point` to the segment from `a` to `b`.
pub(crate) fn segment_distance(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let t = if along.length_squared() > 0. {
        ((point - a).dot(along) / along.length_squared()).clamp(0., 1.)
    } else {
        0.
    };
    point.distance(a + along * t)
}

/// The turn of an arrow's entity that takes its y to `axis`, and its xy plane to face along
/// `toward_camera` as nearly as a plane through the axis can.
fn arrow_rotation(axis: Vec3, toward_camera: Vec3) -> Quat {
    let normal = (toward_camera - axis * toward_camera.dot(axis))
        .try_normalize()
        .unwrap_or_else(|| axis.any_orthonormal_vector());
    Quat::from_mat3(&Mat3::from_cols(axis.cross(normal), axis, normal))
}

/// A picking backend in selection mode: reports the gizmo's handle under each pointer, or the
/// handle a drag holds, ahead of what is under it.
fn pick_gizmo(
    rays: Res<RayMap>,
    cameras: Query<(&Camera, &GlobalTransform), With<PrimaryCamera>>,
    place: Res<GizmoPlace>,
    drag: Res<GizmoDragState>,
    handles: Query<(Entity, &GizmoHandle)>,
    mut hits: MessageWriter<PointerHits>,
) {
    let Some(frame) = place.0 else {
        return;
    };
    for (&id, &ray) in rays.iter() {
        let Ok((camera, camera_transform)) = cameras.get(id.camera) else {
            continue;
        };
        let project = |point: Vec3| camera.world_to_viewport(camera_transform, point).ok();
        let part = match &drag.0 {
            Some(drag) => Some(drag.part),
            // Any point along the ray is where the pointer is on the screen.
            None => project(ray.get_point(1.)).and_then(|cursor| part_at(frame, cursor, project)),
        };
        let Some(entity) = part.and_then(|part| {
            handles
                .iter()
                .find(|(_, handle)| handle.part == part && handle.shown)
                .map(|(entity, _)| entity)
        }) else {
            continue;
        };
        hits.write(PointerHits::new(
            id.pointer,
            vec![(entity, HitData::new(id.camera, 0., None, None))],
            camera.order as f32 + GIZMO_ORDER,
        ));
    }
}

/// The gizmo's handles, hidden until an instance is selected.
fn spawn_gizmo(mut commands: Commands) {
    for part in PARTS {
        commands.spawn((
            Name::new(format!("GizmoHandle:{part:?}")),
            GizmoHandle { part, ..default() },
            OverlayColor {
                base: part.color().into(),
                underlay: 0.4,
            },
            Propagate(RenderLayers::none()),
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(SceneryTool::SelectInstance),
        ));
    }
}

/// The selected instance, as its precinct's asset holds it, or the point chosen in its path.
#[derive(Clone, Debug)]
struct Selected {
    instance: Entity,
    precinct: Entity,
    iid: SceneryInstanceId,
    actor: bool,
    /// In the world: the instance's, or the point's.
    position: Vec3,
    /// In degrees; a point's is 0.
    facing: f32,
    point: Option<ChosenPoint>,
}

/// A path's point, chosen to move in place of its fixture.
#[derive(Clone, Debug, PartialEq)]
struct ChosenPoint {
    index: usize,
    /// The fixture's place, in the world, which the points are offsets from.
    fixture: Vec3,
    /// The path's points, as the fixture has them.
    points: Vec<Vec3>,
}

impl ChosenPoint {
    /// The path's points with the chosen one at `position`, in the world.
    fn moved_to(&self, position: Vec3) -> Vec<Vec3> {
        let mut points = self.points.clone();
        points[self.index] = position - self.fixture;
        points
    }
}

/// The selected instance, if its precinct's asset has it, or its chosen point, if its path has
/// it.
fn selected(
    selection: &ScenerySelection,
    precincts: &Query<&Precinct>,
    edits: &PrecinctEdits,
    paths: &Query<&MarkerPath>,
) -> Option<Selected> {
    let (precinct, instance) = selection.precinct.zip(selection.instance)?;
    let record = precincts.get(precinct).ok()?;
    let (held, facing, actor) = placement_of(edits.get(record)?, &selection.iid)?;
    let position = if actor {
        held
    } else {
        held + precinct_origin(record.coords)
    };
    let mut selected = Selected {
        instance,
        precinct,
        iid: selection.iid.clone(),
        actor,
        position,
        facing,
        point: None,
    };
    if let Some(index) = selection.path_point() {
        let points = paths.get(instance).ok()?.points.clone();
        selected.position = position + *points.get(index)?;
        selected.facing = 0.;
        selected.point = Some(ChosenPoint {
            index,
            fixture: position,
            points,
        });
    }
    Some(selected)
}

/// A drag of one of the gizmo's handles, from the press to the release.
#[derive(Clone, Debug)]
struct GizmoDrag {
    part: GizmoPart,
    /// The instance, where it was at the press.
    of: Selected,
    /// Where the pointer took hold.
    hold: Hold,
    /// Where the drag puts the instance, and its facing.
    position: Vec3,
    facing: f32,
}

/// Where the pointer took hold of a handle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hold {
    /// An arrow, this far along its axis from the instance.
    Along(f32),
    /// The ring, at this angle about the instance, in degrees as a facing is.
    Around(f32),
}

/// The drag in progress, if any.
#[derive(Resource, Default)]
struct GizmoDragState(Option<GizmoDrag>);

/// The normal of the plane through `axis` that faces back along `view` as nearly as any does.
fn axis_plane_normal(view: Vec3, axis: Vec3) -> Vec3 {
    let side = view.cross(axis);
    if side.length_squared() < 1e-6 {
        return axis.any_orthonormal_vector();
    }
    axis.cross(side).normalize()
}

/// How far along `axis` from `origin` `ray` points, where it meets the plane through the axis
/// that faces it.
fn along_axis(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<f32> {
    let normal = axis_plane_normal(*ray.direction, axis);
    let distance = ray.intersect_plane(origin, InfinitePlane3d::new(normal))?;
    Some((ray.get_point(distance) - origin).dot(axis))
}

/// The angle from `center` to `point` about y, in degrees as a facing is: 0 toward +z, 90 toward
/// +x.
fn angle_about(center: Vec3, point: Vec3) -> f32 {
    let offset = point - center;
    offset.x.atan2(offset.z).to_degrees()
}

/// `value` on the move's lattice, unless `free`.
pub(crate) fn snap_coordinate(value: f32, free: bool) -> f32 {
    if free {
        value
    } else {
        (value / MOVE_STEP).round() * MOVE_STEP + 0.
    }
}

/// `facing` turned as the pointer went around from `from` to `to` degrees: snapped as a
/// placement's turn is, or, if `free`, to a tenth of a degree. From 0 to 360.
fn turned(facing: f32, from: f32, to: f32, free: bool) -> f32 {
    let raw = facing + to - from;
    if free {
        ((raw * FREE_TURN_DIVISIONS).round() / FREE_TURN_DIVISIONS).rem_euclid(360.) + 0.
    } else {
        quantize_facing(raw)
    }
}

/// Where the pointer takes hold of `part` along `ray`, for an instance at `start`.
fn hold(part: GizmoPart, ray: Ray3d, start: Vec3) -> Option<Hold> {
    match part.axis() {
        Some(axis) => Some(Hold::Along(along_axis(ray, start, axis)?)),
        None => Some(Hold::Around(angle_about(start, level_point(ray, start.y)?))),
    }
}

/// Where `drag` puts its instance while the pointer is along `ray`, and its facing. A move in x or
/// z puts the coordinate on the lattice, so a move puts an instance placed off it back on it; a
/// move in y keeps the height's offset from the lattice, such as a floor's.
fn dragged(drag: &GizmoDrag, ray: Ray3d, free: bool) -> Option<(Vec3, f32)> {
    let (start, facing) = (drag.of.position, drag.of.facing);
    match drag.hold {
        Hold::Along(from) => {
            let axis = drag.part.axis()?;
            let by = along_axis(ray, start, axis)? - from;
            let mut position = start;
            match drag.part {
                GizmoPart::AxisX => position.x = snap_coordinate(start.x + by, free),
                GizmoPart::AxisZ => position.z = snap_coordinate(start.z + by, free),
                _ => position.y = start.y + snap_coordinate(by, free),
            }
            Some((position, facing))
        }
        Hold::Around(from) => {
            let to = angle_about(start, level_point(ray, start.y)?);
            Some((start, turned(facing, from, to, free)))
        }
    }
}

/// The origin of the precinct at `coords`, in the world.
fn precinct_origin(coords: IVec2) -> Vec3 {
    (coords.as_vec2() * PRECINCT_SIZE_F).extend(0.).xzy()
}

/// The last part of an exemplar's path, for an edit's name.
fn short_name(path: &str) -> &str {
    path.rsplit(['#', '/']).next().unwrap_or(path)
}

/// Where the instance `iid` is in `asset`, as the asset holds it, with its facing in degrees, and
/// whether it is an actor, whose position is in the world.
fn placement_of(asset: &PrecinctAsset, iid: &SceneryInstanceId) -> Option<(Vec3, f32, bool)> {
    if let Some(instance) = asset.scenery.iter().find(|instance| instance.iid == *iid) {
        return Some((instance.position, instance.facing, false));
    }
    let actor = asset.actors.iter().find(|actor| actor.iid == *iid)?;
    Some((actor.position, actor.facing, true))
}

/// The name of the instance `iid` in `asset`, for an edit's.
pub(crate) fn name_of(asset: &PrecinctAsset, iid: &SceneryInstanceId) -> Option<String> {
    if let Some(label) = asset.label(iid) {
        return Some(label.to_string());
    }
    if let Some(instance) = asset.scenery.iter().find(|instance| instance.iid == *iid) {
        let path = asset.scenery_types.get(instance.id)?;
        return Some(short_name(path).to_string());
    }
    let actor = asset.actors.iter().find(|actor| actor.iid == *iid)?;
    Some(short_name(&actor.exemplar).to_string())
}

/// Puts the instance `iid` of `asset` at `position`, as the asset holds it, facing `facing`
/// degrees. Returns the edit's name, "`verb` name"; `None`, refusing it, if the instance is not
/// there or is there already.
fn set_placement(
    asset: &mut PrecinctAsset,
    iid: &SceneryInstanceId,
    position: Vec3,
    facing: f32,
    verb: &str,
) -> Option<Cow<'static, str>> {
    let name = name_of(asset, iid)?;
    let (place, turn) = if let Some(instance) = asset
        .scenery
        .iter_mut()
        .find(|instance| instance.iid == *iid)
    {
        (&mut instance.position, &mut instance.facing)
    } else {
        let actor = asset.actors.iter_mut().find(|actor| actor.iid == *iid)?;
        (&mut actor.position, &mut actor.facing)
    };
    if *place == position && *turn == facing {
        return None;
    }
    *place = position;
    *turn = facing;
    Some(format!("{verb} {name}").into())
}

/// The selection's instances, to show a drag on.
#[derive(SystemParam)]
struct DraggedInstances<'w, 's> {
    element_transforms: Query<'w, 's, &'static mut Transform, With<SceneryElement>>,
    actors: Query<'w, 's, (&'static mut ActorMotion, &'static mut ActorIntent), With<PlacedAs>>,
    precincts: Query<'w, 's, &'static Precinct>,
    paths: Query<'w, 's, &'static MarkerPath>,
    preview: ResMut<'w, PathPreview>,
    commands: Commands<'w, 's>,
}

impl DraggedInstances<'_, '_> {
    /// Shows `of` at `position`, in the world, facing `facing` degrees.
    fn show(&mut self, of: &Selected, position: Vec3, facing: f32) {
        if let Some(point) = &of.point {
            self.preview.show(of.instance, point.moved_to(position));
            return;
        }
        let turn = facing.to_radians();
        if of.actor {
            if let Ok((mut motion, mut intent)) = self.actors.get_mut(of.instance) {
                motion.position = position;
                motion.previous = position;
                motion.facing = turn;
                motion.previous_facing = turn;
                intent.heading = turn;
            }
            return;
        }
        let Ok(precinct) = self.precincts.get(of.precinct) else {
            return;
        };
        if let Ok(mut transform) = self.element_transforms.get_mut(of.instance) {
            let local = Transform::from_translation(position - precinct_origin(precinct.coords))
                .with_rotation(Quat::from_rotation_y(turn));
            transform.set_if_neq(local);
        }
    }

    /// Ends `drag`, showing its instance where it was if `restore`.
    fn end(&mut self, drag: &GizmoDrag, restore: bool) {
        if drag.of.point.is_some() {
            match restore {
                true => self.preview.clear(),
                false => self.preview.settle(),
            }
            return;
        }
        if restore {
            self.show(&drag.of, drag.of.position, drag.of.facing);
        }
        if drag.of.actor {
            if let Ok(mut entity) = self.commands.get_entity(drag.of.instance) {
                entity.remove::<ActorHeld>();
            }
        }
    }
}

/// Where edits put instances: the viewed realm, its physics for landing them, and its precincts.
#[derive(SystemParam)]
struct EditWorld<'w, 's> {
    viewpoint: Res<'w, Viewpoint>,
    realms: Query<'w, 's, &'static RealmPhysics>,
    cache: Res<'w, PrecinctCache>,
    visibility: Query<'w, 's, &'static InheritedVisibility>,
    actor_sizes: Query<
        'w,
        's,
        (Option<&'static CollisionSize>, Option<&'static ActorScale>),
        With<PlacedAs>,
    >,
}

impl EditWorld<'_, '_> {
    /// Where `shapes` of the instance `entity`, at `from` and turned by `rotation`, come to rest
    /// dropped onto what is under it, meeting nothing of its own; `None` if nothing is under it,
    /// or outside the viewed realm.
    fn landing(
        &self,
        shapes: &[(SharedShape, Pose)],
        rotation: Quat,
        from: Vec3,
        entity: Entity,
    ) -> Option<Vec3> {
        let physics = self
            .viewpoint
            .realm
            .and_then(|realm| self.realms.get(realm).ok())?;
        let visible = owner_shown(&self.visibility);
        let shown = |owner: Entity| owner != entity && visible(owner);
        landing(physics.world(), shapes, rotation, from, DROP_FALL, &shown)
    }

    /// Where a path's point at `from` meets what it stands on (`POINT_SURFACES`), straight down
    /// from just above it; `None` if nothing is under it.
    fn point_landing(&self, from: Vec3) -> Option<Vec3> {
        let physics = self
            .viewpoint
            .realm
            .and_then(|realm| self.realms.get(realm).ok())?;
        let ray = Ray3d::new(from + Vec3::Y * POINT_DROP_LIFT, Dir3::NEG_Y);
        surface_point(physics, ray, DROP_FALL, &self.visibility)
    }

    /// Where the actor `entity` comes to rest from `from`, its feet, as its capsule falls.
    fn actor_landing(&self, entity: Entity, from: Vec3) -> Option<Vec3> {
        let (size, scale) = self.actor_sizes.get(entity).ok()?;
        let shape = capsule(size, scale);
        // Its base at the actor's feet, as the actor tool casts it.
        let center = Vec3::Y * (shape.half_height() + shape.radius);
        let shapes = [(
            SharedShape::new(shape),
            Pose::from_translation(center.to_rapier()),
        )];
        self.landing(&shapes, Quat::IDENTITY, from, entity)
    }
}

/// The pointer and keys, for the drag.
#[derive(SystemParam)]
struct GizmoInput<'w, 's> {
    pointers: Res<'w, PointerState>,
    pointer: Res<'w, SceneryPointer>,
    keys: ContextActions<'w, 's, InstanceGizmoKeys>,
    drag_keys: ContextActions<'w, 's, GizmoDragKeys>,
    handles: Query<'w, 's, &'static GizmoHandle>,
    was_pressed: Local<'s, bool>,
}

/// The edit a release or a drop commits, and the selection it keeps.
#[derive(SystemParam)]
struct GizmoEdits<'w, 's> {
    edits: PrecinctEdits<'w, 's>,
    reselect: ResMut<'w, Reselect>,
}

impl GizmoEdits<'_, '_> {
    /// Puts `of` at `position`, in the world, facing `facing` degrees, as an edit named `verb` and
    /// the instance's name: in the precinct there, which a move past the edge changes. A chosen
    /// point stays in its fixture's path, whatever precinct it is over. `false` if refused:
    /// nothing changed, or no precinct is loaded there.
    fn commit(
        &mut self,
        of: &Selected,
        position: Vec3,
        facing: f32,
        verb: &str,
        world: &EditWorld,
        precincts: &Query<&Precinct>,
    ) -> bool {
        let Ok(from) = precincts.get(of.precinct) else {
            return false;
        };
        if let Some(point) = &of.point {
            let points = point.moved_to(position);
            if points == point.points {
                return false;
            }
            let mut changed = false;
            self.edits.edit_precinct(of.precinct, from, |asset| {
                let label = set_path_points(asset, &of.iid, points, |name| {
                    format!("{verb} point {} of {name}", point.index + 1)
                });
                changed = label.is_some();
                label
            });
            return changed;
        }
        let Some((to, coords)) = world
            .viewpoint
            .realm
            .and_then(|realm| precinct_at(&world.cache, realm, position))
        else {
            return false;
        };
        let held = if of.actor {
            position
        } else {
            position - precinct_origin(coords)
        };
        if to == of.precinct {
            let mut changed = false;
            self.edits.edit_precinct(of.precinct, from, |asset| {
                let label = set_placement(asset, &of.iid, held, facing, verb);
                changed = label.is_some();
                label
            });
            // A changed entry spawns the actor again.
            if changed && of.actor {
                self.reselect
                    .wait_for(of.precinct, of.iid.clone(), of.instance);
            }
            return changed;
        }
        let (Some(from_handle), Ok(to_precinct)) = (from.asset.clone(), precincts.get(to)) else {
            return false;
        };
        let Some(to_handle) = self.edits.asset_handle(to, to_precinct) else {
            return false;
        };
        let Some(iid) = self
            .edits
            .move_instance(&from_handle, &to_handle, &of.iid, held, facing)
        else {
            return false;
        };
        self.reselect.wait_for(to, iid, of.instance);
        true
    }
}

/// Starts, follows, cancels and ends drags of the gizmo's handles.
fn drag_gizmo(
    mut input: GizmoInput,
    mut drag: ResMut<GizmoDragState>,
    mut instances: DraggedInstances,
    mut edits: GizmoEdits,
    world: EditWorld,
    selection: Res<ScenerySelection>,
    mut readout: ResMut<TurnReadout>,
) {
    let pressed = input
        .pointers
        .get(PointerId::Mouse, PointerButton::Primary)
        .and_then(|state| {
            state
                .pressing
                .keys()
                .find_map(|entity| input.handles.get(*entity).ok())
        })
        .map(|handle| handle.part);
    let just_pressed = pressed.is_some() && !*input.was_pressed;
    *input.was_pressed = pressed.is_some();
    let free = input.keys.value::<FreeMove>();
    let ray = input.pointer.ray;

    // A drag whose instance or point went, as when an undo took it out, ends where it is.
    if let Some(current) = drag.0.take_if(|drag| {
        selection.instance != Some(drag.of.instance)
            || selection.path_point() != drag.of.point.as_ref().map(|point| point.index)
    }) {
        instances.end(&current, current.of.point.is_some());
    }

    if let Some(part) = pressed.filter(|_| just_pressed && drag.0.is_none()) {
        let of = selected(
            &selection,
            &instances.precincts,
            &edits.edits,
            &instances.paths,
        );
        let started = of.and_then(|of| {
            let hold = hold(part, ray?, of.position)?;
            Some(GizmoDrag {
                part,
                hold,
                position: of.position,
                facing: of.facing,
                of,
            })
        });
        if let Some(started) = started {
            if started.of.actor {
                instances
                    .commands
                    .entity(started.of.instance)
                    .insert(ActorHeld);
            }
            drag.0 = Some(started);
        }
    }

    if input.drag_keys.fired::<CancelGizmoDrag>() {
        if let Some(cancelled) = drag.0.take() {
            instances.end(&cancelled, true);
        }
    }

    if let Some(current) = drag.0.as_mut() {
        if let Some((position, facing)) = ray.and_then(|ray| dragged(current, ray, free)) {
            current.position = position;
            current.facing = facing;
            instances.show(&current.of, position, facing);
        }
    }

    if pressed.is_none() {
        if let Some(mut done) = drag.0.take() {
            // An actor would fall from where it was let go; it is put where it lands. With no
            // ground under it, it would never start its physics, nor have the collider it is
            // picked by, so it goes back.
            let mut grounded = true;
            if done.of.actor && done.part != GizmoPart::Ring {
                match world.actor_landing(done.of.instance, done.position) {
                    Some(landed) => done.position = landed,
                    None => {
                        grounded = false;
                        instances
                            .commands
                            .write_message(ShowToast("Nothing there for it to stand on".into()));
                    }
                }
            }
            let moved =
                grounded && (done.position != done.of.position || done.facing != done.of.facing);
            let verb = if done.part == GizmoPart::Ring {
                "Turn"
            } else {
                "Move"
            };
            let committed = moved
                && edits.commit(
                    &done.of,
                    done.position,
                    done.facing,
                    verb,
                    &world,
                    &instances.precincts,
                );
            instances.end(&done, !committed);
        }
    }

    let turning = drag
        .0
        .as_ref()
        .filter(|drag| drag.part == GizmoPart::Ring)
        .map(|drag| drag.facing);
    if readout.0 != turning {
        readout.0 = turning;
    }
}

/// Drops the selected fixture straight down onto what is under it on End, as one edit, or its
/// chosen point. Actors fall by themselves.
fn drop_selected(
    keys: ContextActions<InstanceGizmoKeys>,
    drag: Res<GizmoDragState>,
    selection: Res<ScenerySelection>,
    colliders: Query<Option<&SceneryColliders>, With<SceneryElement>>,
    precincts: Query<&Precinct>,
    paths: Query<&MarkerPath>,
    mut edits: GizmoEdits,
    world: EditWorld,
) {
    if !keys.fired::<DropInstance>() || drag.0.is_some() {
        return;
    }
    let Some(of) = selected(&selection, &precincts, &edits.edits, &paths).filter(|of| !of.actor)
    else {
        return;
    };
    if of.point.is_some() {
        if let Some(landed) = world.point_landing(of.position) {
            edits.commit(&of, landed, 0., "Drop", &world, &precincts);
        }
        return;
    }
    let shapes: Vec<_> = colliders
        .get(of.instance)
        .ok()
        .flatten()
        .into_iter()
        .flat_map(|colliders| colliders.0.iter())
        .filter_map(ColliderDesc::shape)
        .collect();
    let rotation = Quat::from_rotation_y(of.facing.to_radians());
    let Some(landed) = world.landing(&shapes, rotation, of.position, of.instance) else {
        return;
    };
    if landed != of.position {
        edits.commit(&of, landed, of.facing, "Drop", &world, &precincts);
    }
}

/// Deletes the selected instance on its key, as one edit, leaving its precinct selected; or the
/// chosen point of its path, choosing the one before.
fn delete_selected(
    keys: ContextActions<InstanceGizmoKeys>,
    drag: Res<GizmoDragState>,
    mut selection: ResMut<ScenerySelection>,
    precincts: Query<&Precinct>,
    paths: Query<&MarkerPath>,
    mut edits: PrecinctEdits,
) {
    if !keys.fired::<DeleteInstance>() || drag.0.is_some() {
        return;
    }
    let Some(entity) = selection.precinct.filter(|_| selection.instance.is_some()) else {
        return;
    };
    let Ok(precinct) = precincts.get(entity) else {
        return;
    };
    let iid = selection.iid.clone();
    if let Some(index) = selection.path_point() {
        let Some(path) = selection.instance.and_then(|e| paths.get(e).ok()) else {
            return;
        };
        if index >= path.points.len() {
            return;
        }
        let (points, chosen) = without_point(&path.points, index);
        let mut deleted = false;
        edits.edit_precinct(entity, precinct, |asset| {
            let label = set_path_points(asset, &iid, points, |name| {
                format!("Delete point {} of {name}", index + 1)
            });
            deleted = label.is_some();
            label
        });
        if deleted {
            selection.part = chosen.map(InstancePart::PathPoint);
        }
        return;
    }
    edits.edit_precinct(entity, precinct, |asset| {
        let name = name_of(asset, &iid)?;
        asset.take_instance(&iid)?;
        Some(format!("Delete {name}").into())
    });
    selection.set_if_neq(ScenerySelection {
        precinct: Some(entity),
        ..default()
    });
}

/// Escape chooses no part, which puts the gizmo back on the whole instance.
fn clear_part(keys: ContextActions<ChosenPartKeys>, mut selection: ResMut<ScenerySelection>) {
    if keys.fired::<ClearPart>() && selection.part.is_some() {
        selection.part = None;
    }
}

/// Puts the gizmo on the selected instance, or its chosen point, or where a drag shows it, at its
/// size for the camera's distance, in the viewed realm; lights the handle under the pointer, or
/// the one a drag holds. A point's gizmo has no ring.
/// Shown only while the pointer is over the view, not over UI such as the property panel, so it
/// is out of the way while the panel edits the selection; a drag keeps it wherever the pointer
/// goes.
fn show_gizmo(
    selection: Res<ScenerySelection>,
    drag: Res<GizmoDragState>,
    hover: Res<HoverMap>,
    pointer: Res<SceneryPointer>,
    nodes: Query<(), With<ComputedNode>>,
    cameras: Query<&GlobalTransform, With<PrimaryCamera>>,
    instances: Query<&GlobalTransform, Or<(With<SceneryElement>, With<PlacedAs>)>>,
    paths: Query<&MarkerPath>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    mut place: ResMut<GizmoPlace>,
    mut shown: ResMut<GizmoActive>,
    mut handles: Query<(
        Entity,
        &mut GizmoHandle,
        &mut Transform,
        &mut OverlayColor,
        &mut Propagate<RenderLayers>,
    )>,
) {
    let chosen = selection.path_point();
    let pose = match &drag.0 {
        Some(drag) => Some((drag.position, drag.facing)),
        None => selection.instance.and_then(|entity| {
            let transform = instances.get(entity).ok()?;
            if let Some(index) = chosen {
                let offset = *paths.get(entity).ok()?.points.get(index)?;
                return Some((transform.translation() + offset, 0.));
            }
            let (yaw, _, _) = transform.rotation().to_euler(EulerRot::YXZ);
            Some((transform.translation(), yaw.to_degrees()))
        }),
    };
    let turns = match &drag.0 {
        Some(drag) => drag.of.point.is_none(),
        None => chosen.is_none(),
    };
    // The pointer has a ray only inside the view's viewport, and a hit on UI there (a popup, a
    // dialog) is in front of the view.
    let over_ui = hover
        .get(&PointerId::Mouse)
        .is_some_and(|hits| hits.keys().any(|&entity| nodes.contains(entity)));
    let over_view = pointer.ray.is_some() && !over_ui;
    let camera = cameras.single().ok().map(GlobalTransform::translation);
    let frame = pose
        .filter(|_| over_view || drag.0.is_some())
        .zip(camera)
        .map(|((origin, _), camera)| GizmoFrame::new(origin, camera, turns));
    if place.0 != frame {
        place.0 = frame;
    }
    let layer = viewpoint
        .realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    let hovered = hover.get(&PointerId::Mouse).and_then(|hits| {
        handles
            .iter()
            .find(|(entity, ..)| hits.contains_key(entity))
            .map(|(_, handle, ..)| handle.part)
    });
    let held = drag.0.as_ref().map(|drag| drag.part);
    shown.set_if_neq(GizmoActive { hovered, held });
    let active = held.or(hovered);
    let dragging = held.is_some();
    for (_, mut handle, mut transform, mut color, mut propagate) in &mut handles {
        let part = handle.part;
        let next = GizmoHandle {
            part,
            shown: frame.is_some_and(|frame| frame.turns || part != GizmoPart::Ring),
            facing: pose.map_or(0., |(_, facing)| facing),
        };
        handle.set_if_neq(next);
        if let (Some(frame), Some(camera)) = (frame, camera) {
            let rotation = part.axis().map_or(Quat::IDENTITY, |axis| {
                arrow_rotation(axis, camera - frame.origin)
            });
            transform.set_if_neq(Transform {
                translation: frame.origin,
                rotation,
                scale: Vec3::splat(frame.scale),
            });
        }
        let base = part.color();
        let base = if active == Some(part) {
            base.mix(&css::WHITE.with_alpha(base.alpha), 0.5)
        } else if dragging {
            base.with_alpha(base.alpha * IDLE_ALPHA)
        } else {
            base
        };
        let base = Color::from(base);
        if color.base != base {
            color.base = base;
        }
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}

/// Ends a drag when the tool changes, putting its instance back.
fn end_drag(mut drag: ResMut<GizmoDragState>, mut instances: DraggedInstances) {
    if let Some(current) = drag.0.take() {
        instances.end(&current, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ray from `origin` toward `point`.
    fn ray(origin: Vec3, point: Vec3) -> Ray3d {
        Ray3d::new(origin, Dir3::new(point - origin).unwrap())
    }

    fn drag_of(part: GizmoPart, hold: Hold, start: Vec3) -> GizmoDrag {
        GizmoDrag {
            part,
            of: Selected {
                instance: Entity::PLACEHOLDER,
                precinct: Entity::PLACEHOLDER,
                iid: SceneryInstanceId::None,
                actor: false,
                position: start,
                facing: 0.,
                point: None,
            },
            hold,
            position: start,
            facing: 0.,
        }
    }

    #[test]
    fn an_arrow_moves_along_its_axis_only() {
        let start = Vec3::new(10., 0., 10.);
        let eye = Vec3::new(10., 10., 20.);
        let from = along_axis(ray(eye, start), start, Vec3::X).unwrap();
        assert!(from.abs() < 1e-4);
        let drag = drag_of(GizmoPart::AxisX, Hold::Along(from), start);
        // The pointer goes 1.3 m along the axis.
        let along = start + Vec3::new(1.3, 0., 0.);
        let (position, _) = dragged(&drag, ray(eye, along), true).unwrap();
        assert!(
            (position - Vec3::new(11.3, 0., 10.)).length() < 1e-3,
            "{position}"
        );
        // Snapped, to the quarter meter.
        let (position, _) = dragged(&drag, ray(eye, along), false).unwrap();
        assert_eq!(position, Vec3::new(11.25, 0., 10.));
        // Off the axis, it still moves only along it.
        let (position, _) =
            dragged(&drag, ray(eye, start + Vec3::new(0.7, 0., -0.5)), true).unwrap();
        assert_eq!((position.y, position.z), (0., 10.));
    }

    #[test]
    fn the_y_arrow_keeps_the_heights_offset() {
        let start = Vec3::new(0., 0.1, 0.);
        let eye = Vec3::new(0., 2., 10.);
        let from = along_axis(ray(eye, start), start, Vec3::Y).unwrap();
        let drag = drag_of(GizmoPart::AxisY, Hold::Along(from), start);
        let (position, _) = dragged(&drag, ray(eye, start + Vec3::Y * 0.6), false).unwrap();
        assert!((position.y - 0.6).abs() < 1e-5, "{position}");
    }

    #[test]
    fn the_ring_turns_by_the_pointers_turn() {
        let start = Vec3::ZERO;
        let eye = Vec3::new(0., 10., 0.1);
        let grab = Vec3::new(0., 0., 1.);
        let from = angle_about(start, grab);
        let drag = drag_of(GizmoPart::Ring, Hold::Around(from), start);
        // A quarter turn toward +x, a little short: snapped to the quarter, free to the tenth.
        let point = Vec3::new(1., 0., 0.12);
        let (_, snapped) = dragged(&drag, ray(eye, point), false).unwrap();
        assert_eq!(snapped, 90.);
        let (_, free) = dragged(&drag, ray(eye, point), true).unwrap();
        let expected = (angle_about(start, point) * 10.).round() / 10.;
        assert_eq!(free, expected);
    }

    #[test]
    fn a_turn_wraps_into_a_whole_turn() {
        assert_eq!(turned(350., 0., 20., true), 10.);
        assert_eq!(turned(10., 0., -20., true), 350.);
        assert_eq!(turned(10., 0., -25., false), 345.);
        // Near a quarter turn, the quarter turn.
        assert_eq!(turned(10., 0., -20., false), 0.);
    }

    /// The gizmo seen from straight above, at 10 m: x to the right and z down the screen, in
    /// pixels, 100 to a meter, around (500, 500).
    fn from_above(point: Vec3) -> Option<Vec2> {
        Some(Vec2::new(500., 500.) + point.xz() * 100.)
    }

    #[test]
    fn the_pointer_takes_the_handle_near_it() {
        // At scale 1, an arrow from 0.2 m to 1 m, and the ring 1.25 m out.
        let frame = GizmoFrame {
            origin: Vec3::ZERO,
            scale: 1.,
            turns: true,
        };
        let at = |x: f32, y: f32| part_at(frame, Vec2::new(x, y), from_above);
        assert_eq!(at(580., 503.), Some(GizmoPart::AxisX));
        // Within reach of the arrow's line, off its stroke.
        assert_eq!(at(560., 512.), Some(GizmoPart::AxisX));
        // At the tip, where the arrowhead ends.
        assert_eq!(at(598., 500.), Some(GizmoPart::AxisX));
        assert_eq!(at(503., 580.), Some(GizmoPart::AxisZ));
        assert_eq!(at(625., 500.), Some(GizmoPart::Ring));
        assert_eq!(at(560., 460.), None);
        // A point's gizmo has no ring to take.
        let point = GizmoFrame {
            turns: false,
            ..frame
        };
        assert_eq!(part_at(point, Vec2::new(625., 500.), from_above), None);
    }

    #[test]
    fn an_arrows_plane_holds_its_axis_and_faces_the_camera() {
        let toward = Vec3::new(1., 2., 3.);
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let rotation = arrow_rotation(axis, toward);
            assert!((rotation * Vec3::Y).distance(axis) < 1e-5);
            let normal = rotation * Vec3::Z;
            assert!(normal.dot(axis).abs() < 1e-5);
            assert!(normal.dot(toward) > 0.);
        }
    }

    #[test]
    fn an_edit_moves_and_turns_the_instance_it_names() {
        let mut asset = PrecinctAsset::default();
        let index = asset.add_scenery_type("scenery/furniture.json#Barrel".into());
        asset.add_scenery_element(index, 0., Vec3::new(1., 0., 1.), None);
        let iid = asset.scenery[0].iid.clone();
        assert_eq!(
            set_placement(&mut asset, &iid, Vec3::new(2., 0., 1.), 90., "Move").as_deref(),
            Some("Move Barrel")
        );
        assert_eq!(
            placement_of(&asset, &iid),
            Some((Vec3::new(2., 0., 1.), 90., false))
        );
        // Where it is already, nothing changes.
        assert_eq!(
            set_placement(&mut asset, &iid, Vec3::new(2., 0., 1.), 90., "Move"),
            None
        );
    }
}
