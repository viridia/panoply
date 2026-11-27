//! The goal runtime: the relations that make a tree, and the walk that evaluates it.

use bevy::prelude::*;
use panoply_actors::{ActionFinished, PlayClip};
use panoply_core::GameMode;

/// The root of a goal tree, and the actor whose goal it is. The tree despawns with the actor.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[relationship(relationship_target = Goals)]
pub struct GoalOf(pub Entity);

/// The roots of an actor's goal trees.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
#[relationship_target(relationship = GoalOf, linked_spawn)]
pub struct Goals(Vec<Entity>);

/// A goal's parent in its tree. Not a `ChildOf`, so trees sit outside transform propagation.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[relationship(relationship_target = Subgoals)]
pub struct SubgoalOf(pub Entity);

/// A goal's children, in order, despawned with it.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
#[relationship_target(relationship = SubgoalOf, linked_spawn)]
pub struct Subgoals(Vec<Entity>);

/// What evaluating a goal gives.
#[derive(Debug, Clone, PartialEq)]
pub enum GoalOutcome {
    Satisfied,
    Failed,
    /// Not done, and nothing to do until something changes: the actor's action finishing.
    Waiting,
    /// The body action to take. The goal waits for it once taken.
    Act(BodyAction),
}

/// A body action a goal asks of its actor (R43).
#[derive(Debug, Clone, PartialEq)]
pub enum BodyAction {
    PlayClip(PlayClip),
}

impl BodyAction {
    fn take(self, actor: &mut EntityWorldMut) {
        match self {
            Self::PlayClip(play) => actor.insert(play),
        };
    }

    /// Stops whatever body action a goal gave `actor`, which counts as pre-empting it: no
    /// `ActionFinished`.
    fn stop(actor: &mut EntityWorldMut) {
        actor.remove::<PlayClip>();
    }
}

/// A goal type: a component of its immutable parameters, which requires its state (`State`) and
/// its `GoalEval`.
pub trait Goal: Component {
    /// The goal's state, whose default is the state it starts or restarts in.
    type State: Component + Default;

    /// Evaluates the goal on entity `node`.
    fn evaluate(cx: &mut GoalContext, node: Entity) -> GoalOutcome;
}

/// How to evaluate and reset a goal, whatever its type: function pointers monomorphized for it,
/// so the walk calls a node's goal without knowing its type.
#[derive(Component, Debug, Clone, Copy)]
pub struct GoalEval {
    evaluate: fn(&mut GoalContext, Entity) -> GoalOutcome,
    reset: fn(&mut World, Entity),
}

impl GoalEval {
    pub fn of<T: Goal>() -> Self {
        Self {
            evaluate: T::evaluate,
            reset: |world, node| {
                world.entity_mut(node).insert(T::State::default());
            },
        }
    }
}

/// What a goal's evaluation reads and changes: the world, and the actor whose tree it is.
pub struct GoalContext<'w> {
    pub world: &'w mut World,
    pub actor: Entity,
}

impl GoalContext<'_> {
    /// Evaluates the goal on `node`, a child, so a combinator acts on its result in the same pass.
    pub fn evaluate(&mut self, node: Entity) -> GoalOutcome {
        let Some(eval) = self.world.get::<GoalEval>(node).copied() else {
            warn!("Goal node {node} has no goal");
            return GoalOutcome::Failed;
        };
        (eval.evaluate)(self, node)
    }

    /// Puts the goal on `node`, and every goal under it, back in its starting state.
    pub fn reset(&mut self, node: Entity) {
        if let Some(eval) = self.world.get::<GoalEval>(node).copied() {
            (eval.reset)(self.world, node);
        }
        for child in subgoals(self.world, node) {
            self.reset(child);
        }
    }
}

/// The children of `node`, in order.
pub(crate) fn subgoals(world: &World, node: Entity) -> Vec<Entity> {
    world
        .get::<Subgoals>(node)
        .map(|subgoals| subgoals.iter().collect())
        .unwrap_or_default()
}

/// The roots of `actor`'s trees.
fn roots(world: &World, actor: Entity) -> Vec<Entity> {
    world
        .get::<Goals>(actor)
        .map(|goals| goals.iter().collect())
        .unwrap_or_default()
}

/// Whether goals run: only in play, so actors hold still while they are edited.
fn playing(world: &World) -> bool {
    world
        .get_resource::<State<GameMode>>()
        .is_some_and(|mode| *mode.get() == GameMode::Play)
}

/// Evaluates each of `actor`'s trees, and takes the action one asks for.
fn evaluate_goals(world: &mut World, actor: Entity) {
    if !playing(world) {
        return;
    }
    for root in roots(world, actor) {
        let outcome = GoalContext { world, actor }.evaluate(root);
        match outcome {
            GoalOutcome::Act(action) => {
                if let Ok(mut actor) = world.get_entity_mut(actor) {
                    action.take(&mut actor);
                }
            }
            GoalOutcome::Failed => warn!("Goal {root} of actor {actor} failed"),
            GoalOutcome::Satisfied | GoalOutcome::Waiting => {}
        }
    }
}

/// A tree's first evaluation, once it is rooted at its actor.
pub(crate) fn evaluate_new_tree(
    insert: On<Insert<GoalOf>>,
    roots: Query<&GoalOf>,
    mut commands: Commands,
) {
    if let Ok(&GoalOf(actor)) = roots.get(insert.entity) {
        commands.queue(move |world: &mut World| evaluate_goals(world, actor));
    }
}

/// Evaluates an actor's trees when its body action finishes.
pub(crate) fn evaluate_on_finished(finished: On<ActionFinished>, mut commands: Commands) {
    let actor = finished.entity;
    commands.queue(move |world: &mut World| evaluate_goals(world, actor));
}

/// The actors that have goal trees.
fn actors_with_goals(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<Goals>>()
        .iter(world)
        .collect()
}

/// Entering play starts every actor's trees.
pub(crate) fn start_goals(world: &mut World) {
    for actor in actors_with_goals(world) {
        evaluate_goals(world, actor);
    }
}

/// Leaving play stops every actor's goals: its body action ends, and its trees go back to their
/// starting state, so the next play starts them afresh rather than resuming from whatever was
/// edited in between.
pub(crate) fn stop_goals(world: &mut World) {
    for actor in actors_with_goals(world) {
        BodyAction::stop(&mut world.entity_mut(actor));
        for root in roots(world, actor) {
            GoalContext { world, actor }.reset(root);
        }
    }
}
