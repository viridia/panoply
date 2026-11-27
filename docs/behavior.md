# Behavior

Actors' goals: the goal-tree AI of the three.js version (`engine/src/world/actors/goals`), moved to
ECS, with trees declared in SAGA. The capabilities required are R47 to R55 and R99, in
[actor-requirements.md](actor-requirements.md); the goals still to build wait in X17. The runtime
is the `panoply_goals` crate.

Goals are not aspects, since an actor can hold several goals of one type with different
parameters. Behavior packages own them, and packages are most of what a mod holds
([modding.md](modding.md)).

## Trees

Each actor spawns its own copy of each goal tree its packages hold: an entity per node, joined by
`SubgoalOf` / `Subgoals` (ordered, despawned with the parent, outside transform propagation) and
rooted at the actor by `GoalOf` / `Goals`, so a tree despawns with its actor.

Each goal type is its own component of immutable parameters (`Pose { clip, hold }`), so BSN
writes a tree as components and `Subgoals [ ... ]`. It requires its state, a separate reflected
component whose default is the reset state, and a `GoalEval::of::<Self>()`: function pointers,
monomorphized for the type, that evaluate it and reset it. The walk goes from each root through
those pointers, because a combinator evaluates its active child and acts on the result in the
same pass. The Rust goal types are a closed set per build; mods compose them.

Evaluating a goal gives `Satisfied`, `Failed`, `Waiting`, or `Act`, with the body action to take
(R48). A leaf returns its action rather than taking it, so the root can choose between trees
before any is applied. An actor's trees are evaluated when one is rooted at it and when its body
action finishes (`ActionFinished`), never every tick (R49). A tree that is satisfied or failed
leaves its actor idle.

Goals run only in play (`GameMode::Play`), so actors hold still while they are edited. Entering
play evaluates every actor's trees. Leaving it ends each actor's body action, as a pre-emption,
and resets its trees, so the next play starts them afresh rather than resuming where an edit may
have changed them.

| Goal | Does |
| --- | --- |
| `Sequence { looping }` | Each child once, in order, never back; looping, from the first again, the children reset. A child's failure fails it. A looping sequence that passes through its children without one acting waits, so it cannot spin. |
| `Pose { clip, hold }` | Plays `clip` once, or looped for `hold` seconds, through `PlayClip`; satisfied when it ends. A species without the clip stands for as long. |

Until trees have a format, a few labelled instances have trees written in Rust
(`instance_trees.rs`), each spawned when its actor spawns.

## Behavior packages

A package is a scripted aspect (D107) declaring goal trees, with a say in presence through a field
`Presence` reads (D123). Whether a quest's behavior reaches an actor through a package on it or a
role in the quest is X123. Dialogue and remark sets are listed apart, by the `Dialogues` aspect and
its remarks counterpart (D86). Each actor has one conversation and one remark goal, drawing on the
union of those sets in order, farthest ancestor first. faery's rules carry over
(`engine/src/dialogue/DialogueController.ts`): the greeting is the first intro whose conditions
hold; each state's options are pooled by state name across sets, sorted stably by `order`, and the
first option with a given `id` hides the rest. Remarks choose among the sets whose conditions hold
(R53).

## Format

Trees are SAGA declarations, sketched in [Exemplars.md](Exemplars.md), Packages written in SAGA; a
JSON prototype, the comparison, is in [goal-format.md](goal-format.md).

## Formulas

A goal's condition is a reactive formula; an effect is usually a Rust type with parameters, but can
be a formula. Both are written inline in the tree's declaration, so goals need no formulas
embedded in assets. A node's memo cells live on its entity. Landing re-evaluates an actor's goals
(R37).

## Saves

Saves (PAN82) hold only state that differs from the reset state: per package, a hash of its goal
definition and each changed node's state, keyed by its path of child indices. When the hash no
longer matches (a mod or content edit changed the tree), that package's state is dropped and its
goals restart, which goals tolerate (R50). The same read-out, respawn and write-back is hot
reload, so an edit reaches every actor, and it is what an actor recovers from (R104).

## Testing

Testing is the slow part of authoring behavior: a sequence takes time to play out, and much can go
wrong on the way (the innkeeper going to bed from the kitchen). The runtime needs to start a
character in a given state at a given hour, run time fast, show the active goal and why its
siblings lost (their `when` and priority), and replay a run deterministically.
