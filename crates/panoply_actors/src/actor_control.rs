//! The player's direct control of their actor, relative to the view (R29): W walks away from the
//! camera, S backs toward it, and the actor turns gradually to match. A and D turn the camera, and
//! with it the way W walks; standing still, the player can turn the view to see their actor's front
//! or what lies behind it. The arrows do the same as WASD.

use bevy::{app::Propagate, prelude::*};
use bevy_action_map::prelude::*;
use panoply_core::{GotoLocation, Realm, Viewpoint};
use std::f32::consts::PI;

use crate::{Actor, ActorMotion, TravelTo};

/// Walk away from the camera (+) or back toward it (-).
#[derive(InputAction)]
#[action(path = "player.walk", output = f32, intent = Analog1)]
pub struct PlayerWalk;

/// Turn the camera left (-) or right (+), read by the viewpoint's own turning (`move_viewpoint`).
#[derive(InputAction)]
#[action(path = "player.turn", output = f32, intent = Analog1)]
pub struct PlayerTurn;

/// Go at a faster pace while held.
#[derive(InputAction)]
#[action(path = "player.fast", output = bool, intent = Button)]
pub struct PlayerFast;

/// Act on what lies ahead of the player's actor.
#[derive(InputAction)]
#[action(path = "player.interact", output = bool, intent = Button)]
pub struct PlayerInteract;

/// The player's control of their actor.
#[derive(InputContext)]
#[context(path = "player.control", tick = Render)]
pub struct PlayerControl;

/// Paces, in m/s (R17).
const WALK_SPEED: f32 = 1.5;
const TROT_SPEED: f32 = 3.0;
const JOG_SPEED: f32 = 3.5;

/// Marks the actor the player controls.
#[derive(Component, Debug, Default)]
pub struct PlayerActor;

/// The movement asked of an actor: turn toward `heading` and travel along the actor's facing at
/// `speed`. Headings are as `Viewpoint::azimuth`: 0 faces +z, PI/2 faces +x.
#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component)]
pub struct ActorIntent {
    pub heading: f32,
    /// In m/s; negative walks backward.
    pub speed: f32,
}

/// Turns the player's keys into their actor's intent. Pressing W or S has the camera follow the
/// actor again after the free camera left it, and pre-empts travel (R45).
pub(crate) fn drive_player(
    mut commands: Commands,
    input: ContextActions<PlayerControl>,
    mut viewpoint: ResMut<Viewpoint>,
    mut players: Query<(Entity, &ActorMotion, &mut ActorIntent, Has<TravelTo>), With<PlayerActor>>,
) {
    let Ok((entity, motion, mut intent, travelling)) = players.single_mut() else {
        return;
    };
    let walk = input.value::<PlayerWalk>();
    let forward = walk > 0.0;
    let back = walk < 0.0;
    if travelling {
        if !(forward || back) {
            return;
        }
        commands.entity(entity).remove::<TravelTo>();
    }
    let fast = input.value::<PlayerFast>();
    let away = viewpoint.azimuth;
    *intent = if forward {
        ActorIntent {
            heading: away,
            speed: if fast { JOG_SPEED } else { TROT_SPEED },
        }
    } else if back {
        // An actor facing the camera walks toward it rather than turning around to back up.
        if angle_between(motion.facing, away).abs() < PI * 0.5 {
            ActorIntent {
                heading: away,
                speed: -WALK_SPEED,
            }
        } else {
            ActorIntent {
                heading: away + PI,
                speed: TROT_SPEED,
            }
        }
    } else {
        ActorIntent {
            heading: motion.facing,
            speed: 0.0,
        }
    };
    if forward || back {
        viewpoint.follow = Some(entity);
    }
}

/// Entering play, the camera follows the player's actor again, wherever the free camera left it.
pub(crate) fn follow_player(
    mut viewpoint: ResMut<Viewpoint>,
    players: Query<Entity, With<PlayerActor>>,
) {
    if let Ok(player) = players.single() {
        viewpoint.follow = Some(player);
    }
}

/// In play, Go To moves the player's actor: after `goto_location` has moved the view there, the
/// actor joins it and the view follows the actor again. Without its `ActorMotion` the actor waits,
/// as a new one does, until the ground there has loaded (`start_actor_physics`).
pub(crate) fn goto_player(
    mut gotos: MessageReader<GotoLocation>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut Actor, &mut Transform), With<PlayerActor>>,
    realms: Query<(Entity, &Realm)>,
) {
    let Some(goto) = gotos.read().last() else {
        return;
    };
    let Ok((player, mut actor, mut transform)) = players.single_mut() else {
        return;
    };
    let Some((realm_entity, realm)) = realms.iter().find(|(_, realm)| realm.name == goto.realm)
    else {
        return;
    };
    actor.realm = realm_entity;
    transform.translation = goto.position;
    commands
        .entity(player)
        .remove::<(ActorMotion, TravelTo)>()
        .insert((ActorIntent::default(), Propagate(realm.layer.clone())));
    // After `goto_location`'s command, which leaves the view following nothing.
    commands.queue(move |world: &mut World| {
        world.resource_mut::<Viewpoint>().follow = Some(player);
    });
}

/// The signed angle from `from` to `to`, in (-PI, PI].
pub(crate) fn angle_between(from: f32, to: f32) -> f32 {
    PI - (from - to + PI).rem_euclid(2.0 * PI)
}
