//! Body actions that move an actor (R43). Travel takes an actor in a straight line to a point at a
//! pace, through the character controller, so it is stopped by what is in the way.

use bevy::prelude::*;

use crate::{actor_control::angle_between, ActorIntent, ActorMotion};

/// How near the target, across the ground, counts as there, in m.
const ARRIVAL_DISTANCE: f32 = 0.05;

/// Travel to `target` at `speed`, in m/s. Removed when the actor arrives, which triggers
/// `ActionFinished`, or when something pre-empts it, which does not (R45). Height is ignored: the
/// actor stays on the ground.
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct TravelTo {
    pub target: Vec3,
    pub speed: f32,
}

/// Triggered on an actor when its body action finishes (R45).
#[derive(EntityEvent, Debug)]
pub struct ActionFinished {
    pub entity: Entity,
}

/// Steers travelling actors toward their targets, one fixed tick at a time, before they move.
pub(crate) fn travel_actors(
    mut commands: Commands,
    mut actors: Query<(Entity, &TravelTo, &ActorMotion, &mut ActorIntent)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.timestep().as_secs_f32();
    for (entity, travel, motion, mut intent) in &mut actors {
        let offset = (travel.target - motion.position).xz();
        let distance = offset.length();
        if distance < ARRIVAL_DISTANCE {
            *intent = ActorIntent {
                heading: motion.facing,
                speed: 0.0,
            };
            commands.entity(entity).remove::<TravelTo>();
            commands.trigger(ActionFinished { entity });
            continue;
        }
        let heading = offset.x.atan2(offset.y);
        // Slowed while turning, to a stop when the target is abeam or behind, so the actor turns
        // toward a near target rather than circling it. Never past the target in one tick.
        let alignment = angle_between(motion.facing, heading).cos().max(0.0);
        let speed = (travel.speed * alignment).min(distance / dt);
        *intent = ActorIntent { heading, speed };
    }
}
