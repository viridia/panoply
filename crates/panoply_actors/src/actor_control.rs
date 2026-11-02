//! The player's direct control of their actor, relative to the view (R29): W walks away from the
//! camera, S backs toward it, and the actor turns gradually to match. A and D turn the camera
//! (`ViewpointControl`), and with it the way W walks.

use bevy::prelude::*;
use panoply_core::Viewpoint;
use std::f32::consts::PI;

use crate::{ActorMotion, TravelTo};

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
    keys: Res<ButtonInput<KeyCode>>,
    mut viewpoint: ResMut<Viewpoint>,
    mut players: Query<(Entity, &ActorMotion, &mut ActorIntent, Has<TravelTo>), With<PlayerActor>>,
) {
    let Ok((entity, motion, mut intent, travelling)) = players.single_mut() else {
        return;
    };
    let forward = keys.pressed(KeyCode::KeyW);
    let back = keys.pressed(KeyCode::KeyS);
    if travelling {
        if !(forward || back) {
            return;
        }
        commands.entity(entity).remove::<TravelTo>();
    }
    let fast = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
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

/// The signed angle from `from` to `to`, in (-PI, PI].
pub(crate) fn angle_between(from: f32, to: f32) -> f32 {
    PI - (from - to + PI).rem_euclid(2.0 * PI)
}
