//! Scenery mode's selection mode, as faery's `SelectSceneryInstanceMode`: a click selects the wall,
//! fixture or actor under the pointer, with its precinct, and a click on anything else selects the
//! precinct there and no instance. The pointer comes through picking (D94): a backend casts its ray
//! as play's picking casts it (D61), at walls, fixtures, actors, floors and ground alike, and
//! reports the instance it first meets, so nothing is picked through a wall, and a click on a
//! widget or dialog in front of the view is theirs; markers stop the ray too, the handles of things
//! the player never sees (D93). A click on the ground arrives on the parcel `parcel_selection`'s
//! backend reports. The player's actor stops the ray but is no precinct's, so it is not picked.
//!
//! The instance under the pointer is outlined, and the selected one more brightly: the footprint of
//! each collider it is picked by, on its base, or for an actor its capsule's.
//!
//! The selection is the instance's entity, which a precinct keeps when its edits leave the
//! instance in place, and its id, which finds it again when the entity goes: an actor is spawned
//! again when its entry changes, as a rename changes it. It is not saved, as faery's was not. A
//! part of the instance may be chosen within it, a path's point (`path_edit`), which the gizmo then
//! moves (D129).

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        events::PointerClick,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
        PickingSystems,
    },
    prelude::*,
};
use panoply_actors::{capsule, ActorScale, CollisionSize, HomePrecinct, PlacedAs, PrecinctActors};
use panoply_core::{
    realm_physics::{collision_groups, owner_shown},
    PrimaryCamera, Realm, RealmPhysics, Viewpoint,
};
use panoply_overlays::{
    AddOverlay, DrawPlane, OverlayColor, OverlayShape, PolygonOptions, ShapeBuilder,
};
use panoply_precinct::{PrecinctAsset, SceneryInstanceId};
use bevy_action_map::prelude::*;
use panoply_scenery::{
    precinct::Precinct, scenery_element::SceneryElement, MarkerPath, SceneryColliders,
};
use panoply_terrain::Parcel;

use crate::{
    instance_gizmo::{ExtendPath, InstanceGizmoKeys},
    mode_selector::{EditorMode, SceneryTool},
    scenery_pointer::{SceneryPointer, SceneryPointerSystems},
};

/// How far from the camera the pointer's ray can pick, in meters.
const PICK_RANGE: f32 = 500.;

/// How far above an instance's base its outline is drawn, so the ground does not hide it.
const OUTLINE_LIFT: f32 = 0.02;

/// The half extents of the square outlining an element with no colliders, as faery's.
const DEFAULT_HALF: f32 = 0.5;

/// The points around an actor's outline.
const ACTOR_SEGMENTS: usize = 32;

/// The hovered instance's outline, and a hovered part's tint.
pub(crate) const HOVERED_TINT: Srgba = css::YELLOW;

/// The selected instance's outline, and a chosen part's tint.
pub(crate) const SELECTED_TINT: Srgba = Srgba::rgb(1., 1., 0.8);

/// The instance selected in scenery mode, its precinct, and the part of it chosen.
#[derive(Resource, Reflect, Clone, PartialEq, Debug, Default)]
#[reflect(Resource)]
pub struct ScenerySelection {
    /// The precinct's entity.
    pub precinct: Option<Entity>,
    /// The instance's entity: a scenery element or an actor of the precinct.
    pub instance: Option<Entity>,
    /// The instance's id, which finds it again if its entity goes.
    #[reflect(ignore)]
    pub iid: SceneryInstanceId,
    /// The part of the instance chosen, which the gizmo moves in its place.
    pub part: Option<InstancePart>,
}

/// A part of an instance, chosen within it.
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug)]
pub enum InstancePart {
    /// A point of its [`MarkerPath`], by index.
    PathPoint(usize),
}

impl ScenerySelection {
    /// The instance `entity`, of id `iid`, in `precinct`, no part of it chosen.
    pub fn instance(precinct: Entity, entity: Entity, iid: SceneryInstanceId) -> Self {
        Self {
            precinct: Some(precinct),
            instance: Some(entity),
            iid,
            part: None,
        }
    }

    /// The index of the path point chosen, if one is.
    pub fn path_point(&self) -> Option<usize> {
        match self.part? {
            InstancePart::PathPoint(index) => Some(index),
        }
    }
}

/// How many frames [`Reselect`] waits for its instance to be spawned.
const RESELECT_FRAMES: u32 = 120;

/// An instance an edit spawns again, to select once it is there: an actor whose entry a move
/// changed, or an instance moved to another precinct, whose id there may be new.
#[derive(Resource, Default)]
pub(crate) struct Reselect(Option<PendingReselect>);

struct PendingReselect {
    precinct: Entity,
    iid: SceneryInstanceId,
    /// The entity it replaces, which may stand in a while with the same id.
    replaced: Entity,
    frames: u32,
}

impl Reselect {
    /// Selects the instance `iid` of `precinct` once it is spawned in place of `replaced`.
    pub(crate) fn wait_for(&mut self, precinct: Entity, iid: SceneryInstanceId, replaced: Entity) {
        self.0 = Some(PendingReselect {
            precinct,
            iid,
            replaced,
            frames: RESELECT_FRAMES,
        });
    }
}

/// Which instance an [`InstanceOutline`] follows.
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OutlineOf {
    #[default]
    Hovered,
    Selected,
}

/// An instance's outline: closed polygons in the world's x and z, at a height. Hidden while empty.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct InstanceOutline {
    pub of: OutlineOf,
    pub polygons: Vec<Vec<Vec2>>,
    pub height: f32,
}

impl OverlayShape for InstanceOutline {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let width = match self.of {
            OutlineOf::Hovered => 0.04,
            OutlineOf::Selected => 0.07,
        };
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(self.height + OUTLINE_LIFT)
            .with_stroke_width(width);
        for polygon in &self.polygons {
            builder.stroke_polygon(
                polygon,
                PolygonOptions {
                    closed: true,
                    ..default()
                },
            );
        }
    }
}

pub struct InstanceSelectionPlugin;

impl Plugin for InstanceSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenerySelection>()
            .init_resource::<Reselect>()
            .register_type::<ScenerySelection>()
            .register_type::<InstanceOutline>()
            .add_overlay::<InstanceOutline>()
            .add_systems(
                PreUpdate,
                pick_scenery
                    .in_set(PickingSystems::Backend)
                    .run_if(in_state(SceneryTool::SelectInstance)),
            )
            .add_systems(OnEnter(SceneryTool::SelectInstance), spawn_tool)
            .add_systems(
                Update,
                (
                    follow_selection.run_if(in_state(EditorMode::Scenery)),
                    reselect.run_if(in_state(EditorMode::Scenery)),
                    show_outlines.run_if(in_state(SceneryTool::SelectInstance)),
                )
                    .chain()
                    .after(SceneryPointerSystems),
            );
    }
}

/// The click observer, and the outlines of the hovered and the selected instance.
fn spawn_tool(mut commands: Commands) {
    commands.spawn((
        Name::new("SelectInstance"),
        Observer::new(select_clicked),
        DespawnOnExit(SceneryTool::SelectInstance),
    ));
    let outlines = [
        (OutlineOf::Hovered, HOVERED_TINT.with_alpha(0.7)),
        (OutlineOf::Selected, SELECTED_TINT.with_alpha(0.9)),
    ];
    for (of, color) in outlines {
        commands.spawn((
            Name::new(format!("InstanceOutline:{of:?}")),
            InstanceOutline { of, ..default() },
            OverlayColor {
                base: color.into(),
                underlay: 0.5,
            },
            Propagate(RenderLayers::none()),
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(SceneryTool::SelectInstance),
        ));
    }
}

/// Keeps the selection on its instance, and clears it when the precinct leaves the cache. A keyed
/// instance is found by its key while the precinct's asset has it, so an edit that spawns an actor
/// again keeps it; the old entity stands in until the new one is there. Otherwise, as after an
/// undone edit that gave a key, or for an internal id, which a reload numbers anew (D87), the
/// entity is followed and its id taken.
///
/// The part chosen stays while its instance does, until the instance's path changes and no longer
/// has the point, as after an undo. A path an edit lengthens changes a frame or so after the edit
/// chooses its new point, which waits for it.
fn follow_selection(
    mut selection: ResMut<ScenerySelection>,
    precincts: Query<(&Precinct, Option<&Children>, Option<&PrecinctActors>)>,
    elements: Query<&SceneryElement>,
    actors: Query<&PlacedAs>,
    paths: Query<Ref<MarkerPath>>,
    assets: Res<Assets<PrecinctAsset>>,
) {
    let Some(precinct) = selection.precinct else {
        return;
    };
    let Ok((record, children, placed)) = precincts.get(precinct) else {
        selection.set_if_neq(ScenerySelection::default());
        return;
    };
    let Some(selected) = selection.instance else {
        return;
    };
    let iid_of = |entity: Entity| {
        elements
            .get(entity)
            .map(|element| element.iid.clone())
            .or_else(|_| actors.get(entity).map(|actor| actor.iid().clone()))
            .ok()
    };
    let iid = selection.iid.clone();
    let keyed = iid.key().is_some()
        && record
            .asset_in(&assets)
            .is_some_and(|asset| asset.has_instance(&iid));
    let next = if keyed {
        children
            .into_iter()
            .flatten()
            .copied()
            .chain(placed.into_iter().flat_map(|actors| actors.iter()))
            .find(|&entity| iid_of(entity).as_ref() == Some(&iid))
            .or_else(|| iid_of(selected).map(|_| selected))
            .map(|entity| (entity, iid))
    } else {
        iid_of(selected).map(|iid| (selected, iid))
    };
    let followed = match next {
        Some((entity, iid)) => {
            let part = selection.part.filter(|_| entity == selected).filter(|part| {
                let InstancePart::PathPoint(index) = *part;
                paths
                    .get(entity)
                    .is_ok_and(|path| !path.is_changed() || index < path.points.len())
            });
            ScenerySelection {
                part,
                ..ScenerySelection::instance(precinct, entity, iid)
            }
        }
        None => ScenerySelection {
            precinct: Some(precinct),
            ..default()
        },
    };
    selection.set_if_neq(followed);
}

/// Selects the instance [`Reselect`] waits for, once its precinct has spawned it; gives up after a
/// while, as when the edit was undone first.
fn reselect(
    mut pending: ResMut<Reselect>,
    precincts: Query<(Option<&Children>, Option<&PrecinctActors>)>,
    elements: Query<&SceneryElement>,
    actors: Query<&PlacedAs>,
    mut selection: ResMut<ScenerySelection>,
) {
    let Some(wait) = pending.0.as_mut() else {
        return;
    };
    let found = precincts
        .get(wait.precinct)
        .ok()
        .and_then(|(children, placed)| {
            children
                .into_iter()
                .flatten()
                .copied()
                .chain(placed.into_iter().flat_map(|actors| actors.iter()))
                .filter(|&entity| entity != wait.replaced)
                .find(|&entity| {
                    let iid = elements
                        .get(entity)
                        .map(|element| &element.iid)
                        .or_else(|_| actors.get(entity).map(PlacedAs::iid));
                    iid.is_ok_and(|iid| *iid == wait.iid)
                })
        });
    if let Some(entity) = found {
        selection.set_if_neq(ScenerySelection::instance(
            wait.precinct,
            entity,
            wait.iid.clone(),
        ));
        pending.0 = None;
        return;
    }
    wait.frames = wait.frames.saturating_sub(1);
    if wait.frames == 0 {
        pending.0 = None;
    }
}

/// What selection mode picks: scenery elements, and the actors precincts place.
type Instance = Or<(With<SceneryElement>, With<PlacedAs>)>;

/// A picking backend in selection mode. Casts each pointer's ray in the view realm and reports the
/// instance it first meets, if what it first meets is one, with the point hit.
fn pick_scenery(
    rays: Res<RayMap>,
    cameras: Query<&Camera, With<PrimaryCamera>>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&RealmPhysics>,
    instances: Query<(), Instance>,
    visibility: Query<&InheritedVisibility>,
    mut hits: MessageWriter<PointerHits>,
) {
    let shown = owner_shown(&visibility);
    let Some(physics) = viewpoint.realm.and_then(|realm| realms.get(realm).ok()) else {
        return;
    };
    for (&id, &ray) in rays.iter() {
        let Ok(camera) = cameras.get(id.camera) else {
            continue;
        };
        let Some((instance, distance)) = physics
            .pick(ray, PICK_RANGE, !collision_groups::CANOPY, &shown)
            .filter(|(entity, _)| instances.contains(*entity))
        else {
            continue;
        };
        let hit = HitData::new(id.camera, distance, Some(ray.get_point(distance)), None);
        hits.write(PointerHits::new(
            id.pointer,
            vec![(instance, hit)],
            camera.order as f32,
        ));
    }
}

/// Observer while in selection mode: a click on a scenery element or a placed actor selects it and
/// its precinct; a click on the ground, a parcel, selects the precinct where the pointer meets the
/// tier's plane, and no instance. Every other target is left alone, whatever it is: a click on a
/// widget that does not consume it bubbles up to the window, which is no UI node either. A click it
/// handles goes no further. A Ctrl+click while a path is selected is `path_edit`'s.
fn select_clicked(
    mut click: On<PointerClick>,
    elements: Query<(&SceneryElement, &ChildOf)>,
    actors: Query<(&PlacedAs, &HomePrecinct)>,
    parcels: Query<(), With<Parcel>>,
    paths: Query<(), With<MarkerPath>>,
    keys: ContextActions<InstanceGizmoKeys>,
    pointer: Res<SceneryPointer>,
    mut selection: ResMut<ScenerySelection>,
) {
    let target = click.event_target();
    if click.button != PointerButton::Primary {
        return;
    }
    let extending = keys.value::<ExtendPath>()
        && selection
            .instance
            .is_some_and(|entity| paths.contains(entity));
    if extending {
        return;
    }
    let chosen = if let Ok((element, parent)) = elements.get(target) {
        ScenerySelection::instance(parent.parent(), target, element.iid.clone())
    } else if let Ok((placed, home)) = actors.get(target) {
        ScenerySelection::instance(home.0, target, placed.iid().clone())
    } else if parcels.contains(target) {
        ScenerySelection {
            precinct: pointer.precinct.map(|(entity, _)| entity),
            ..default()
        }
    } else {
        return;
    };
    click.propagate(false);
    selection.set_if_neq(chosen);
}

/// Outlines the hovered and the selected instance, in the view realm's layer.
fn show_outlines(
    hover: Res<HoverMap>,
    instances: Query<(), Instance>,
    selection: Res<ScenerySelection>,
    elements: Query<(&SceneryElement, Option<&SceneryColliders>)>,
    actors: Query<(&Transform, Option<&CollisionSize>, Option<&ActorScale>), With<PlacedAs>>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    mut outlines: Query<(&mut InstanceOutline, &mut Propagate<RenderLayers>)>,
) {
    let layer = viewpoint
        .realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    let outline_of = |entity: Option<Entity>| -> (Vec<Vec<Vec2>>, f32) {
        let Some(entity) = entity else {
            return default();
        };
        if let Ok((element, colliders)) = elements.get(entity) {
            return (element_outline(element, colliders), element.position.y);
        }
        if let Ok((transform, size, scale)) = actors.get(entity) {
            let radius = capsule(size, scale).radius;
            let center = transform.translation;
            return (vec![circle(center.xz(), radius)], center.y);
        }
        default()
    };
    let hovered = hover.get(&PointerId::Mouse).and_then(|hits| {
        hits.keys()
            .copied()
            .find(|&entity| instances.contains(entity))
    });
    for (mut outline, mut propagate) in &mut outlines {
        let entity = match outline.of {
            OutlineOf::Hovered => hovered.filter(|&e| selection.instance != Some(e)),
            OutlineOf::Selected => selection.instance,
        };
        let (polygons, height) = outline_of(entity);
        let next = InstanceOutline {
            of: outline.of,
            polygons,
            height,
        };
        outline.set_if_neq(next);
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}

/// The footprints of an element's picked colliders, placed in the world; a square if it has none.
fn element_outline(
    element: &SceneryElement,
    colliders: Option<&SceneryColliders>,
) -> Vec<Vec<Vec2>> {
    let mut polygons: Vec<Vec<Vec2>> = colliders
        .into_iter()
        .flat_map(|colliders| colliders.0.iter())
        .filter_map(|collider| collider.footprint())
        .collect();
    if polygons.is_empty() {
        let half = DEFAULT_HALF;
        polygons.push(vec![
            Vec2::new(-half, -half),
            Vec2::new(half, -half),
            Vec2::new(half, half),
            Vec2::new(-half, half),
        ]);
    }
    // A turn about y by the facing, in x and z.
    let rotation = Rot2::radians(-element.facing);
    let origin = element.position.xz();
    for polygon in &mut polygons {
        for point in polygon.iter_mut() {
            *point = origin + rotation * *point;
        }
    }
    polygons
}

/// A circle of `radius` around `center`, as points.
fn circle(center: Vec2, radius: f32) -> Vec<Vec2> {
    (0..ACTOR_SEGMENTS)
        .map(|i| {
            let angle = i as f32 / ACTOR_SEGMENTS as f32 * std::f32::consts::TAU;
            center + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An element's outline turns with its facing as its colliders do.
    #[test]
    fn an_element_outline_turns_with_the_element() {
        let element = SceneryElement {
            facing: std::f32::consts::FRAC_PI_2,
            position: Vec3::new(10., 1., 20.),
            ..default()
        };
        let outline = element_outline(&element, None);
        // A quarter turn about y takes +x to -z.
        let turned = Quat::from_rotation_y(element.facing) * Vec3::new(0.5, 0., -0.5);
        let expected = element.position.xz() + turned.xz();
        assert!(
            outline[0].iter().any(|p| p.distance(expected) < 1e-5),
            "{outline:?} lacks {expected}"
        );
    }
}
