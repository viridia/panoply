//! The actors precincts place (D24): spawned when their home precinct's asset loads, and
//! despawned with it.

use bevy::{app::Propagate, prelude::*};
use panoply_exemplar::{CatalogServer, Exemplar};
use panoply_precinct::{ActorInstanceData, PrecinctAsset, SceneryInstanceId};
use panoply_scenery::precinct::Precinct;

use crate::{Actor, ActorIntent, ActorRebuildAspects};

/// The precinct that placed an actor, which owns it: the actor despawns with the precinct.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[relationship(relationship_target = PrecinctActors)]
pub struct HomePrecinct(pub Entity);

/// The actors a precinct placed.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
#[relationship_target(relationship = HomePrecinct, linked_spawn)]
pub struct PrecinctActors(Vec<Entity>);

/// Marks a precinct whose asset loaded or changed, until its actors are spawned.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct ReadPrecinctActors;

pub(crate) fn mark_precinct_actors(
    mut commands: Commands,
    precincts: Query<(Entity, &Precinct)>,
    mut events: MessageReader<AssetEvent<PrecinctAsset>>,
) {
    for event in events.read() {
        let (AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id }) = event
        else {
            continue;
        };
        if let Some((entity, _)) = precincts.iter().find(|(_, p)| p.asset.id() == *id) {
            commands.entity(entity).insert(ReadPrecinctActors);
        }
    }
}

/// What placed an actor: its entry in its precinct's asset, without its aspects. An edit to the
/// precinct that leaves the entry as it was leaves the actor alone.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct PlacedAs {
    iid: SceneryInstanceId,
    exemplar: String,
    position: Vec3,
    facing: f32,
}

impl PlacedAs {
    /// The actor's id in its precinct.
    pub fn iid(&self) -> &SceneryInstanceId {
        &self.iid
    }

    /// The actor's exemplar, a `file#Label` path.
    pub fn exemplar(&self) -> &str {
        &self.exemplar
    }

    fn of(data: &ActorInstanceData) -> Self {
        Self {
            iid: data.iid.clone(),
            exemplar: data.exemplar.clone(),
            position: data.position,
            facing: data.facing,
        }
    }
}

/// Spawns a marked precinct's actors once their exemplar catalogs have loaded. When the asset
/// changes, an actor whose entry changed, or is gone, is replaced, so it starts again where the
/// asset places it; the others are left as they are, so an edit elsewhere in the precinct does not
/// restart them.
// TODO(PAN28e): an edit to an actor's aspects alone replaces nothing, as entries are compared
// without them.
pub(crate) fn spawn_precinct_actors(
    mut commands: Commands,
    precincts: Query<(Entity, &Precinct, Option<&PrecinctActors>), With<ReadPrecinctActors>>,
    placed: Query<&PlacedAs>,
    assets: Res<Assets<PrecinctAsset>>,
    mut exemplars: CatalogServer,
) {
    for (entity, precinct, actors) in &precincts {
        let Some(asset) = assets.get(&precinct.asset) else {
            continue;
        };
        let Some(handles) =
            exemplars.get_all::<Exemplar>(asset.actors.iter().map(|actor| &actor.exemplar))
        else {
            continue;
        };
        commands.entity(entity).remove::<ReadPrecinctActors>();
        let mut existing: Vec<(Entity, &PlacedAs)> = actors
            .into_iter()
            .flat_map(|actors| actors.iter())
            .filter_map(|actor| Some((actor, placed.get(actor).ok()?)))
            .collect();
        for (data, exemplar) in asset.actors.iter().zip(handles) {
            let placement = PlacedAs::of(data);
            if let Some(index) = existing.iter().position(|(_, p)| **p == placement) {
                existing.swap_remove(index);
                continue;
            }
            let facing = data.facing.to_radians();
            let label = data
                .exemplar
                .rsplit_once('#')
                .map_or(data.exemplar.as_str(), |(_, label)| label);
            commands.spawn((
                Name::new(format!("Actor:{label}")),
                Actor {
                    exemplar,
                    realm: precinct.realm,
                },
                HomePrecinct(entity),
                placement,
                // Without an intent, an actor would turn to heading 0.
                ActorIntent {
                    heading: facing,
                    speed: 0.0,
                },
                data.aspects.clone(),
                ActorRebuildAspects,
                Transform::from_translation(data.position)
                    .with_rotation(Quat::from_rotation_y(facing)),
                Visibility::default(),
                Propagate(precinct.render_layer.clone()),
            ));
        }
        for (actor, _) in existing {
            commands.entity(actor).despawn();
        }
    }
}
