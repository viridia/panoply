//! TODO(PAN134d): trees are declared in SAGA. Until then, the trees of a few labelled instances
//! are written here, each spawned when its actor spawns.

use bevy::{ecs::system::EntityCommands, prelude::*};
use panoply_actors::{HomePrecinct, PlacedAs};
use panoply_precinct::PrecinctAsset;
use panoply_scenery::precinct::Precinct;

use crate::{GoalOf, Pose, Sequence, Subgoals};

/// Spawns the tree of the instance labelled `label`, if it has one.
fn spawn_tree<'a>(commands: &'a mut Commands, label: &str) -> Option<EntityCommands<'a>> {
    match label {
        "playground.guard.test" => Some(commands.spawn_scene(bsn! {
            Sequence { looping: true }
            Subgoals [
                Pose { clip: "Dance1", hold: {Some(4.0)} }
                --
                Pose { clip: "StandGesture" }
            ]
        })),
        _ => None,
    }
}

/// Gives a precinct's actor its tree, by its label in its precinct's asset.
pub(crate) fn spawn_instance_tree(
    add: On<Add<PlacedAs>>,
    mut commands: Commands,
    actors: Query<(&PlacedAs, &HomePrecinct)>,
    precincts: Query<&Precinct>,
    assets: Res<Assets<PrecinctAsset>>,
) {
    let Ok((placed, home)) = actors.get(add.entity) else {
        return;
    };
    let Some(label) = precincts
        .get(home.0)
        .ok()
        .and_then(|precinct| precinct.asset_in(&assets))
        .and_then(|asset| asset.label(placed.iid()))
    else {
        return;
    };
    if let Some(mut tree) = spawn_tree(&mut commands, label) {
        tree.insert(GoalOf(add.entity));
    }
}
