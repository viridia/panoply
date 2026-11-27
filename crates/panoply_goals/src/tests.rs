use bevy::{
    prelude::*,
    state::{app::StatesPlugin, state::StateTransition},
};
use panoply_actors::{ActionFinished, PlayClip};
use panoply_core::GameMode;

use crate::{GoalOf, PanoplyGoalsPlugin, Pose, Sequence, Subgoals};

/// An app in `mode`, with an actor.
fn world_with_actor(mode: GameMode) -> (App, Entity) {
    let mut app = App::new();
    // What spawning a scene needs, and the mode.
    app.add_plugins((
        TaskPoolPlugin::default(),
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        StatesPlugin,
        PanoplyGoalsPlugin,
    ))
    .insert_state(mode);
    app.world_mut().run_schedule(StateTransition);
    let actor = app.world_mut().spawn_empty().id();
    (app, actor)
}

fn set_mode(app: &mut App, mode: GameMode) {
    let world = app.world_mut();
    world.resource_mut::<NextState<GameMode>>().set(mode);
    world.run_schedule(StateTransition);
    world.flush();
}

fn guard_tree() -> impl Scene {
    bsn! {
        Sequence { looping: true }
        Subgoals [
            Pose { clip: "Dance1", hold: {Some(4.0)} }
            --
            Pose { clip: "StandGesture" }
        ]
    }
}

fn root(app: &mut App, actor: Entity, tree: impl Scene) {
    let world = app.world_mut();
    world.spawn_scene(tree).unwrap().insert(GoalOf(actor));
    world.flush();
}

/// The actor's `PlayClip`, as its clip and hold.
fn playing(app: &App, actor: Entity) -> Option<(String, Option<f32>)> {
    app.world()
        .get::<PlayClip>(actor)
        .map(|play| (play.clip.clone(), play.hold))
}

/// Ends the actor's action, as the body does.
fn finish(app: &mut App, actor: Entity) {
    let world = app.world_mut();
    world.entity_mut(actor).remove::<PlayClip>();
    world.trigger(ActionFinished { entity: actor });
    world.flush();
}

fn dance() -> Option<(String, Option<f32>)> {
    Some(("Dance1".to_owned(), Some(4.0)))
}

fn gesture() -> Option<(String, Option<f32>)> {
    Some(("StandGesture".to_owned(), None))
}

#[test]
fn looping_sequence_of_poses() {
    let (mut app, actor) = world_with_actor(GameMode::Play);
    root(&mut app, actor, guard_tree());
    let (dance, gesture) = (dance(), gesture());
    assert_eq!(playing(&app, actor), dance);
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), gesture);
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), dance);
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), gesture);
}

#[test]
fn goals_wait_in_editing() {
    let (mut app, actor) = world_with_actor(GameMode::Edit);
    root(&mut app, actor, guard_tree());
    assert_eq!(playing(&app, actor), None);
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), None);
    set_mode(&mut app, GameMode::Play);
    assert_eq!(playing(&app, actor), dance());
}

#[test]
fn leaving_play_resets_goals() {
    let (mut app, actor) = world_with_actor(GameMode::Play);
    root(&mut app, actor, guard_tree());
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), gesture());
    set_mode(&mut app, GameMode::Edit);
    assert_eq!(playing(&app, actor), None, "still posing in editing");
    // Back in play, from the start, not the gesture it had reached.
    set_mode(&mut app, GameMode::Play);
    assert_eq!(playing(&app, actor), dance());
}

#[test]
fn sequence_ends_satisfied() {
    let (mut app, actor) = world_with_actor(GameMode::Play);
    root(
        &mut app,
        actor,
        bsn! {
            Sequence
            Subgoals [ Pose { clip: "Wave" } ]
        },
    );
    assert_eq!(playing(&app, actor), Some(("Wave".to_owned(), None)));
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), None);
    // Another action finishing leaves it done.
    finish(&mut app, actor);
    assert_eq!(playing(&app, actor), None);
}

#[test]
fn empty_looping_sequence_waits() {
    let (mut app, actor) = world_with_actor(GameMode::Play);
    root(&mut app, actor, bsn! { Sequence { looping: true } });
    assert_eq!(playing(&app, actor), None);
}

#[test]
fn tree_despawns_with_its_actor() {
    let (mut app, actor) = world_with_actor(GameMode::Play);
    root(
        &mut app,
        actor,
        bsn! {
            Sequence
            Subgoals [ Pose { clip: "Wave" } ]
        },
    );
    let world = app.world_mut();
    world.entity_mut(actor).despawn();
    let left = world.query::<&Pose>().iter(world).count();
    assert_eq!(left, 0, "{left} goals left");
}
