//! Editing a selected [`MarkerPath`]'s points in selection mode, as Blender edits a curve's, cut
//! down to one point at a time and with no edit mode: while a path is selected, a click on one of
//! its point's pins chooses that point, which the gizmo then moves in place of the fixture
//! (`instance_gizmo`), and Ctrl+click puts a new point after the chosen one, or at the end, where
//! the pointer meets the world, and chooses it. Delete takes the chosen point out, and End drops it
//! onto what is under it. Alt+Left and Alt+Right choose the point before and after, around the
//! path; Tab is left for widgets, and the arrows alone drive the camera.
//!
//! A point's pin is taken ahead of the gizmo, which stands on the fixture, as a point near the
//! fixture would otherwise lie under its arrows; the chosen point's pin is left to the gizmo on it.
//!
//! A path's points are offsets from its fixture, which is no point of the route: the fixture's pin
//! moves them all. An edit of a point writes the whole path into the instance's own aspects,
//! overriding the path it inherits, as the property panel's pencil would.

use std::{borrow::Cow, sync::Arc};

use bevy::{
    ecs::system::SystemParam,
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        events::PointerClick,
        pointer::PointerButton,
        PickingSystems,
    },
    prelude::*,
};
use bevy_action_map::prelude::*;
use panoply_core::{
    realm_physics::{collision_groups, owner_shown},
    PrimaryCamera, RealmPhysics, Viewpoint,
};
use panoply_exemplar::Aspect;
use panoply_precinct::{PrecinctAsset, SceneryInstanceId};
use panoply_scenery::{precinct::Precinct, MarkerPath};
use rapier3d::prelude::Group;

use crate::{
    instance_gizmo::{
        name_of, segment_distance, snap_coordinate, ExtendPath, FreeMove, InstanceGizmoKeys,
        GIZMO_ORDER, HIT_PIXELS,
    },
    instance_selection::{InstancePart, ScenerySelection},
    marker_overlays::{pin_top, PathPointPin},
    mode_selector::SceneryTool,
    precinct_edit::PrecinctEdits,
    scenery_pointer::SceneryPointer,
};

/// Above the gizmo's hits, and below its UI's, which Bevy reports half an order above the camera's.
const POINT_ORDER: f32 = GIZMO_ORDER * 1.5;

/// How far a cast for a point looks, in meters.
const POINT_RANGE: f32 = 500.;

/// What a path's point stands on: the ground, floors, walls, fixtures and trees, but not markers,
/// actors, which move away, or the canopy over a forest, which only catches what falls.
pub(crate) const POINT_SURFACES: Group = collision_groups::TERRAIN
    .union(collision_groups::FLOOR)
    .union(collision_groups::WALL)
    .union(collision_groups::FIXTURE)
    .union(collision_groups::FLORA);

/// Where `ray` first meets what a path's point stands on, among what is shown.
pub(crate) fn surface_point(
    physics: &RealmPhysics,
    ray: Ray3d,
    range: f32,
    visibility: &Query<&InheritedVisibility>,
) -> Option<Vec3> {
    let (_, distance) = physics.pick(ray, range, POINT_SURFACES, owner_shown(visibility))?;
    Some(ray.get_point(distance))
}

/// The keys that step through the selected path's points.
#[derive(InputContext)]
#[context(path = "editor.path_points", tick = Render, priority = 100)]
pub struct PathPointKeys;

/// Choose the point before the chosen one, or the last.
#[derive(InputAction)]
#[action(path = "editor.path_points.previous", output = bool, intent = Button)]
pub struct PreviousPathPoint;

/// Choose the point after the chosen one, or the first.
#[derive(InputAction)]
#[action(path = "editor.path_points.next", output = bool, intent = Button)]
pub struct NextPathPoint;

pub struct PathEditPlugin;

impl Plugin for PathEditPlugin {
    fn build(&self, app: &mut App) {
        app.add_context::<PathPointKeys>(|context| {
            context.active_if(
                |selection: Res<ScenerySelection>,
                 paths: Query<(), With<MarkerPath>>,
                 tool: Option<Res<State<SceneryTool>>>| {
                    tool.is_some_and(|tool| *tool.get() == SceneryTool::SelectInstance)
                        && selection
                            .instance
                            .is_some_and(|entity| paths.contains(entity))
                },
            );
            // Above the camera's arrows, and consumed, so Alt+Left does not turn it too.
            context
                .bind::<PreviousPathPoint>(KeyCode::ArrowLeft)
                .with(ModifierKey::Alt)
                .consume();
            context
                .bind::<NextPathPoint>(KeyCode::ArrowRight)
                .with(ModifierKey::Alt)
                .consume();
        })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Name::new("PathPointKeys"), PathPointKeys));
        })
        .add_systems(
            PreUpdate,
            pick_path_points
                .in_set(PickingSystems::Backend)
                .run_if(in_state(SceneryTool::SelectInstance)),
        )
        .add_systems(
            Update,
            step_path_points.run_if(in_state(SceneryTool::SelectInstance)),
        )
        .add_systems(OnEnter(SceneryTool::SelectInstance), spawn_observer);
    }
}

/// Alt+Left and Alt+Right choose the point before and after the chosen one, around the path; with
/// none chosen, the last and the first.
fn step_path_points(
    keys: ContextActions<PathPointKeys>,
    paths: Query<&MarkerPath>,
    mut selection: ResMut<ScenerySelection>,
) {
    let by = keys.fired::<NextPathPoint>() as i32 - keys.fired::<PreviousPathPoint>() as i32;
    if by == 0 {
        return;
    }
    let Some(path) = selection.instance.and_then(|entity| paths.get(entity).ok()) else {
        return;
    };
    if let Some(index) = stepped(selection.path_point(), by, path.points.len()) {
        selection.part = Some(InstancePart::PathPoint(index));
    }
}

/// The point `by` steps from `chosen` among `count`, around the path: from none, the first going
/// forward and the last going back.
fn stepped(chosen: Option<usize>, by: i32, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let count = count as i32;
    let from = match chosen {
        Some(index) => index as i32,
        None if by > 0 => -1,
        None => count,
    };
    Some((from + by).rem_euclid(count) as usize)
}

fn spawn_observer(mut commands: Commands) {
    commands.spawn((
        Name::new("EditPath"),
        Observer::new(path_clicked),
        DespawnOnExit(SceneryTool::SelectInstance),
    ));
}

/// A picking backend in selection mode: reports the selected path's point whose pin is nearest the
/// pointer on the screen, within reach, but for the chosen one.
fn pick_path_points(
    rays: Res<RayMap>,
    cameras: Query<(&Camera, &GlobalTransform), With<PrimaryCamera>>,
    selection: Res<ScenerySelection>,
    pins: Query<(Entity, &PathPointPin, &GlobalTransform)>,
    mut hits: MessageWriter<PointerHits>,
) {
    let Some(fixture) = selection.instance else {
        return;
    };
    for (&id, &ray) in rays.iter() {
        let Ok((camera, camera_transform)) = cameras.get(id.camera) else {
            continue;
        };
        let project = |point: Vec3| camera.world_to_viewport(camera_transform, point).ok();
        // Any point along the ray is where the pointer is on the screen.
        let Some(cursor) = project(ray.get_point(1.)) else {
            continue;
        };
        let rotation = camera_transform.rotation();
        let nearest = pins
            .iter()
            .filter(|(_, pin, _)| {
                pin.fixture == fixture && selection.path_point() != Some(pin.index)
            })
            .filter_map(|(entity, _, transform)| {
                let tip = transform.translation();
                let (a, b) = (project(tip)?, project(pin_top(tip, rotation))?);
                Some((segment_distance(cursor, a, b), entity, tip))
            })
            .filter(|(distance, ..)| *distance <= HIT_PIXELS)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, entity, tip)) = nearest else {
            continue;
        };
        hits.write(PointerHits::new(
            id.pointer,
            vec![(entity, HitData::new(id.camera, 0., Some(tip), None))],
            camera.order as f32 + POINT_ORDER,
        ));
    }
}

/// Where the pointer meets what a path's point stands on, in the viewed realm.
#[derive(SystemParam)]
struct PointerSurface<'w, 's> {
    pointer: Res<'w, SceneryPointer>,
    viewpoint: Res<'w, Viewpoint>,
    realms: Query<'w, 's, &'static RealmPhysics>,
    visibility: Query<'w, 's, &'static InheritedVisibility>,
}

impl PointerSurface<'_, '_> {
    /// Where the pointer's ray first meets the ground, a floor, a wall, a fixture or a tree; else
    /// where it meets the tier's level plane. Picking's own hit leaves floors out, so it would put
    /// a point on the terrain under a floor.
    fn point(&self) -> Option<Vec3> {
        let physics = self.viewpoint.realm.and_then(|r| self.realms.get(r).ok());
        self.pointer
            .ray
            .zip(physics)
            .and_then(|(ray, physics)| surface_point(physics, ray, POINT_RANGE, &self.visibility))
            .or(self.pointer.point)
    }
}

/// Observer while in selection mode: a click on a point's pin of the selected path chooses the
/// point; a Ctrl+click elsewhere adds one there. A click it handles goes no further.
fn path_clicked(
    mut click: On<PointerClick>,
    pins: Query<&PathPointPin>,
    paths: Query<(&MarkerPath, &GlobalTransform)>,
    precincts: Query<&Precinct>,
    keys: ContextActions<InstanceGizmoKeys>,
    surface: PointerSurface,
    mut selection: ResMut<ScenerySelection>,
    mut edits: PrecinctEdits,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let Some(fixture) = selection.instance else {
        return;
    };
    if let Ok(pin) = pins.get(click.event_target()) {
        if pin.fixture == fixture {
            click.propagate(false);
            selection.part = Some(InstancePart::PathPoint(pin.index));
        }
        return;
    }
    if !keys.value::<ExtendPath>() {
        return;
    }
    let (Ok((path, origin)), Some(precinct)) = (paths.get(fixture), selection.precinct) else {
        return;
    };
    let Some(at) = surface.point() else {
        return;
    };
    click.propagate(false);
    let Ok(record) = precincts.get(precinct) else {
        return;
    };
    let free = keys.value::<FreeMove>();
    let at = Vec3::new(
        snap_coordinate(at.x, free),
        at.y,
        snap_coordinate(at.z, free),
    );
    let index = selection
        .path_point()
        .map_or(path.points.len(), |chosen| chosen + 1)
        .min(path.points.len());
    let mut points = path.points.clone();
    points.insert(index, at - origin.translation());
    let iid = selection.iid.clone();
    let mut added = false;
    edits.edit_precinct(precinct, record, |asset| {
        let label = set_path_points(asset, &iid, points, |name| {
            format!("Add point {} to {name}", index + 1)
        });
        added = label.is_some();
        label
    });
    if added {
        selection.part = Some(InstancePart::PathPoint(index));
    }
}

/// Gives the scenery instance `iid` of `asset` a path of `points`, among its own aspects, in place
/// of the one it has or inherits. Returns the edit's name, `label` given the instance's name;
/// `None`, refusing it, if the instance is not there.
pub(crate) fn set_path_points(
    asset: &mut PrecinctAsset,
    iid: &SceneryInstanceId,
    points: Vec<Vec3>,
    label: impl FnOnce(&str) -> String,
) -> Option<Cow<'static, str>> {
    let name = name_of(asset, iid)?;
    let instance = asset
        .scenery
        .iter_mut()
        .find(|instance| instance.iid == *iid)?;
    let path: Arc<dyn Aspect> = Arc::new(MarkerPath { points });
    let aspects = &mut instance.aspects.0;
    match aspects.iter_mut().find(|aspect| aspect.key() == path.key()) {
        Some(own) => *own = path,
        None => aspects.push(path),
    }
    Some(label(&name).into())
}

/// `points` without the one at `index`, and the point to choose after: the one before it, or the
/// first, or none if no point is left.
pub(crate) fn without_point(points: &[Vec3], index: usize) -> (Vec<Vec3>, Option<usize>) {
    let mut left = points.to_vec();
    left.remove(index);
    let chosen = (!left.is_empty()).then(|| index.saturating_sub(1));
    (left, chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_overrides_the_inherited_path_then_replaces_its_own() {
        let mut asset = PrecinctAsset::default();
        let index = asset.add_scenery_type("exemplars/furnishings.exem.json#WaypointsList".into());
        asset.add_scenery_element(index, 0., Vec3::ZERO, None);
        let iid = asset.scenery[0].iid.clone();
        let label = set_path_points(&mut asset, &iid, vec![Vec3::X], |name| {
            format!("Add point 1 to {name}")
        });
        assert_eq!(label.as_deref(), Some("Add point 1 to WaypointsList"));
        set_path_points(&mut asset, &iid, vec![Vec3::X, Vec3::Z], |_| String::new());
        let aspects = &asset.scenery[0].aspects.0;
        assert_eq!(aspects.len(), 1);
        let path = aspects[0]
            .as_reflect()
            .downcast_ref::<MarkerPath>()
            .unwrap();
        assert_eq!(path.points, vec![Vec3::X, Vec3::Z]);
    }

    #[test]
    fn stepping_goes_around_the_path() {
        assert_eq!(stepped(Some(2), 1, 3), Some(0));
        assert_eq!(stepped(Some(0), -1, 3), Some(2));
        assert_eq!(stepped(Some(1), 1, 3), Some(2));
        // From none, the first forward and the last back.
        assert_eq!(stepped(None, 1, 3), Some(0));
        assert_eq!(stepped(None, -1, 3), Some(2));
        assert_eq!(stepped(None, 1, 0), None);
    }

    #[test]
    fn deleting_a_point_chooses_the_one_before() {
        let points = [Vec3::X, Vec3::Y, Vec3::Z];
        assert_eq!(without_point(&points, 1), (vec![Vec3::X, Vec3::Z], Some(0)));
        assert_eq!(without_point(&points, 0), (vec![Vec3::Y, Vec3::Z], Some(0)));
        assert_eq!(without_point(&[Vec3::X], 0), (vec![], None));
    }
}
