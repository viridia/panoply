//! The actors precincts place (D24): spawned when their home precinct's asset loads, and
//! despawned with it.

use bevy::{app::Propagate, prelude::*};
use panoply_exemplar::ExemplarServer;
use panoply_precinct::PrecinctAsset;
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

/// Spawns a marked precinct's actors once their exemplar catalogs have loaded. A changed asset
/// replaces the actors, so each starts again where the asset places it.
pub(crate) fn spawn_precinct_actors(
    mut commands: Commands,
    precincts: Query<(Entity, &Precinct, Has<PrecinctActors>), With<ReadPrecinctActors>>,
    assets: Res<Assets<PrecinctAsset>>,
    mut exemplars: ExemplarServer,
) {
    for (entity, precinct, has_actors) in &precincts {
        let Some(asset) = assets.get(&precinct.asset) else {
            continue;
        };
        let Some(handles) = exemplars.get_all(asset.actors.iter().map(|actor| &actor.exemplar))
        else {
            continue;
        };
        commands.entity(entity).remove::<ReadPrecinctActors>();
        if has_actors {
            commands.entity(entity).despawn_related::<PrecinctActors>();
        }
        for (data, exemplar) in asset.actors.iter().zip(handles) {
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
    }
}
