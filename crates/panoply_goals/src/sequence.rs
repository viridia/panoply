use bevy::prelude::*;

use crate::{goal::subgoals, Goal, GoalContext, GoalEval, GoalOutcome};

/// Each child once, in order, never back (R51); with `looping`, from the first again once the last
/// is satisfied, the children reset. A child that fails fails the sequence.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
#[require(SequenceState, GoalEval = GoalEval::of::<Sequence>())]
pub struct Sequence {
    pub looping: bool,
}

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct SequenceState {
    /// The child being pursued.
    pub index: usize,
}

impl Goal for Sequence {
    type State = SequenceState;

    fn evaluate(cx: &mut GoalContext, node: Entity) -> GoalOutcome {
        let Some(looping) = cx
            .world
            .get::<Sequence>(node)
            .map(|sequence| sequence.looping)
        else {
            return GoalOutcome::Failed;
        };
        let children = subgoals(cx.world, node);
        let set_index = |cx: &mut GoalContext, index| {
            if let Some(mut state) = cx.world.get_mut::<SequenceState>(node) {
                state.index = index;
            }
        };
        let mut wrapped = false;
        loop {
            let index = cx
                .world
                .get::<SequenceState>(node)
                .map_or(0, |state| state.index);
            let Some(&child) = children.get(index) else {
                if !looping {
                    return GoalOutcome::Satisfied;
                }
                // A second pass in one evaluation would be a third, and so on: nothing in the
                // loop waits or acts.
                if wrapped {
                    warn!("Looping sequence {node} passed through its goals without acting");
                    return GoalOutcome::Waiting;
                }
                wrapped = true;
                for &child in &children {
                    cx.reset(child);
                }
                set_index(cx, 0);
                continue;
            };
            match cx.evaluate(child) {
                GoalOutcome::Satisfied => set_index(cx, index + 1),
                outcome => return outcome,
            }
        }
    }
}
