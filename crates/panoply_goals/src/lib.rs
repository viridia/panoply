//! Actors' goals ([behavior.md](../../../docs/behavior.md)): trees of goal entities, rooted at an
//! actor, that choose its body actions. A tree is evaluated when the actor's action finishes, never
//! every tick (R49), and only in play.

use bevy::prelude::*;
use panoply_core::GameMode;

mod goal;
mod instance_trees;
mod pose;
mod sequence;

pub use goal::{
    BodyAction, Goal, GoalContext, GoalEval, GoalOf, GoalOutcome, Goals, SubgoalOf, Subgoals,
};
pub use pose::{Pose, PoseState};
pub use sequence::{Sequence, SequenceState};

pub struct PanoplyGoalsPlugin;

impl Plugin for PanoplyGoalsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GoalOf>()
            .register_type::<Goals>()
            .register_type::<SubgoalOf>()
            .register_type::<Subgoals>()
            .register_type::<Pose>()
            .register_type::<PoseState>()
            .register_type::<Sequence>()
            .register_type::<SequenceState>()
            .add_systems(OnEnter(GameMode::Play), goal::start_goals)
            .add_systems(OnExit(GameMode::Play), goal::stop_goals)
            .add_observer(goal::evaluate_new_tree)
            .add_observer(goal::evaluate_on_finished)
            .add_observer(instance_trees::spawn_instance_tree);
    }
}

#[cfg(test)]
mod tests;
