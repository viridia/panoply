//! Builds precincts' navigation meshes off the main thread: each simulated precinct's (R8), and
//! any other a [`NavMeshWanted`] asks for (the editor's navigation view), nearest the viewpoint
//! first. The main thread only captures the input (`NavInputCapture`), copying descriptors and
//! triangles; the task builds the mesh.
//!
//! A mesh is rebuilt after what it was built from changes: each marker that rebuilds a
//! precinct's scenery, floors or terrain effects, or its parcels' ground or flora, marks the mesh
//! stale ([`MarkNavMeshStale`]), and the build starts once the precinct and its parcels are ready
//! again. A stale build in flight is dropped, which cancels its task, so its result never lands;
//! the old mesh stays until the new one does. A precinct without a mesh is not ready for routes.

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};
use panoply_core::Viewpoint;
use panoply_scenery::{
    floor_region::RebuildFloorMesh,
    precinct::{Precinct, ReadPrecinctData},
    scenery_element::SceneryElementRebuildColliders,
    RebuildTerrainFxVertexAttrs, SceneryPending, SimulatedPrecinct, PRECINCT_SIZE_F,
};
use panoply_terrain::{Parcel, ParcelFloraChanged, RebuildParcelGroundMesh, TerrainPending};

use crate::{clipping, input::PARCELS_PER_SIDE, NavInputCapture, NavMesh, PrecinctNavMesh};

/// Asks for a precinct's mesh to be built, once the precinct and its parcels are ready.
#[derive(Component, Debug, Default)]
#[component(storage = "SparseSet")]
pub struct NavMeshWanted;

/// A precinct's mesh being built.
#[derive(Component)]
struct NavMeshBuild(Task<NavMesh>);

pub(crate) fn plugin(app: &mut App) {
    app.add_observer(stale_precinct::<ReadPrecinctData>)
        .add_observer(stale_precinct::<RebuildTerrainFxVertexAttrs>)
        .add_observer(stale_ancestor::<SceneryElementRebuildColliders>)
        .add_observer(stale_ancestor::<RebuildFloorMesh>)
        .add_observer(stale_parcel::<RebuildParcelGroundMesh>)
        .add_observer(stale_parcel::<ParcelFloraChanged>)
        .add_systems(
            Update,
            (want_simulated, start_builds, finish_builds).chain(),
        );
}

/// Marks a precinct's mesh stale: a build in flight is dropped, and a new one is wanted. Does
/// nothing to a precinct that has no mesh and none coming, so a precinct loading does not get one
/// it was not asked for. What changes a precinct's navigation without the markers [`plugin`]
/// watches queues this.
pub struct MarkNavMeshStale(pub Entity);

impl Command for MarkNavMeshStale {
    type Out = ();

    fn apply(self, world: &mut World) {
        let Ok(mut precinct) = world.get_entity_mut(self.0) else {
            return;
        };
        let has_mesh = precinct.contains::<PrecinctNavMesh>() || precinct.contains::<NavMeshBuild>();
        if has_mesh {
            precinct.remove::<NavMeshBuild>().insert(NavMeshWanted);
        }
    }
}

/// A marker inserted on the precinct itself.
fn stale_precinct<M: Component>(insert: On<Insert<M>>, mut commands: Commands) {
    commands.queue(MarkNavMeshStale(insert.entity));
}

/// A marker inserted on something under a precinct: an element, or a floor region under its tier.
fn stale_ancestor<M: Component>(
    insert: On<Insert<M>>,
    parents: Query<&ChildOf>,
    precincts: Query<(), With<Precinct>>,
    mut commands: Commands,
) {
    if let Some(precinct) = parents
        .iter_ancestors(insert.entity)
        .find(|&e| precincts.contains(e))
    {
        commands.queue(MarkNavMeshStale(precinct));
    }
}

/// A marker inserted on a parcel: the precinct it lies in.
fn stale_parcel<M: Component>(
    insert: On<Insert<M>>,
    parcels: Query<&Parcel>,
    precincts: Query<(Entity, &Precinct)>,
    mut commands: Commands,
) {
    let Ok(parcel) = parcels.get(insert.entity) else {
        return;
    };
    let coords = parcel.coords.div_euclid(IVec2::splat(PARCELS_PER_SIDE));
    if let Some((precinct, _)) = precincts
        .iter()
        .find(|(_, p)| p.realm == parcel.realm && p.coords == coords)
    {
        commands.queue(MarkNavMeshStale(precinct));
    }
}

/// A precinct that comes into the simulation box without a mesh wants one.
fn want_simulated(
    precincts: Query<
        Entity,
        (
            Added<SimulatedPrecinct>,
            Without<PrecinctNavMesh>,
            Without<NavMeshBuild>,
        ),
    >,
    mut commands: Commands,
) {
    for entity in &precincts {
        commands.entity(entity).insert(NavMeshWanted);
    }
}

/// What [`start_builds`] reads to tell whether a precinct is ready.
#[derive(SystemParam)]
struct Readiness<'w, 's> {
    scenery: SceneryPending<'w, 's>,
    terrain: TerrainPending<'w, 's>,
}

impl Readiness<'_, '_> {
    /// Whether the colliders navigation reads are ready, as physics on that ground waits
    /// (`area_waiting`).
    fn ready(&self, precinct: &Precinct) -> bool {
        let area = Rect::from_corners(
            precinct.coords.as_vec2() * PRECINCT_SIZE_F,
            (precinct.coords + 1).as_vec2() * PRECINCT_SIZE_F,
        );
        // The rect's far edges lie on the next parcels' near edges; pulled in so they are not
        // waited on.
        let area = area.inflate(-0.5);
        self.scenery.area_waiting(precinct.realm, area).is_none()
            && self.terrain.area_waiting(precinct.realm, area).is_none()
    }
}

/// Captures the input of each wanting precinct in the viewpoint's realm that is ready, nearest
/// the viewpoint first, and builds its mesh off the main thread. A precinct seen through a portal,
/// in another realm, waits until the viewpoint is in its realm.
fn start_builds(
    viewpoint: Res<Viewpoint>,
    wanting: Query<(Entity, &Precinct), With<NavMeshWanted>>,
    readiness: Readiness,
    capture: NavInputCapture,
    mut commands: Commands,
) {
    if wanting.is_empty() {
        return;
    }
    let distance = |precinct: &Precinct| {
        let middle = (precinct.coords.as_vec2() + 0.5) * PRECINCT_SIZE_F;
        middle.distance_squared(viewpoint.position.xz())
    };
    let mut ready: Vec<_> = wanting
        .iter()
        .filter(|(_, p)| viewpoint.realm == Some(p.realm) && readiness.ready(p))
        .collect();
    ready.sort_by(|a, b| distance(a.1).total_cmp(&distance(b.1)));
    for (entity, precinct) in ready {
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<NavMeshWanted>();
        match capture.capture(precinct.realm, precinct.coords) {
            Ok(input) => {
                let task =
                    AsyncComputeTaskPool::get().spawn(async move { clipping::build(&input) });
                entity_commands.insert(NavMeshBuild(task));
            }
            Err(error) => warn!("Precinct {}: no navigation input: {error}", precinct.coords),
        }
    }
}

/// Puts each finished mesh on its precinct.
fn finish_builds(mut builds: Query<(Entity, &mut NavMeshBuild)>, mut commands: Commands) {
    for (entity, mut build) in &mut builds {
        if let Some(mesh) = check_ready(&mut build.0) {
            commands
                .entity(entity)
                .remove::<NavMeshBuild>()
                .insert(PrecinctNavMesh(mesh));
        }
    }
}
