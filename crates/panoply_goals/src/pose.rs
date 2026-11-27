use bevy::prelude::*;
use panoply_actors::PlayClip;

use crate::{BodyAction, Goal, GoalContext, GoalEval, GoalOutcome};

/// Play `clip` once, or looped for `hold` seconds. Satisfied when it ends.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
#[require(PoseState, GoalEval = GoalEval::of::<Pose>())]
pub struct Pose {
    pub clip: String,
    #[reflect(default)]
    pub hold: Option<f32>,
}

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct PoseState {
    /// Its `PlayClip` has been asked for.
    pub started: bool,
}

impl Goal for Pose {
    type State = PoseState;

    fn evaluate(cx: &mut GoalContext, node: Entity) -> GoalOutcome {
        let Some(pose) = cx.world.get::<Pose>(node).cloned() else {
            return GoalOutcome::Failed;
        };
        let playing = cx.world.get::<PlayClip>(cx.actor).is_some();
        let Some(mut state) = cx.world.get_mut::<PoseState>(node) else {
            return GoalOutcome::Failed;
        };
        if !state.started {
            state.started = true;
            GoalOutcome::Act(BodyAction::PlayClip(PlayClip {
                clip: pose.clip,
                hold: pose.hold,
            }))
        } else if playing {
            GoalOutcome::Waiting
        } else {
            GoalOutcome::Satisfied
        }
    }
}
