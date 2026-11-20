//! Actors pushing each other aside (R23, R28): a repulsion between actors, not a collision. Once
//! their capsules come within `AURA_GAP`, two actors are given a push apart, shared by their weights, which
//! `move_actors` adds to their travel, so a pushed actor is still swept against the world. The
//! push is a velocity eased toward its target rather than a force, so it does not jitter.

use std::f32::consts::PI;

use bevy::prelude::*;
use panoply_core::Viewpoint;

use crate::{
    actor_physics::capsule, Actor, ActorHeld, ActorMotion, ActorResting, ActorScale, CollisionSize,
};

/// How an actor stands its ground against being pushed. Not an aspect: being immovable, as a guard
/// blocking a door is, or an actor a scripted goal sets to block a path for a scene, is a state its
/// goals set.
#[derive(Component, Reflect, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub enum ActorFooting {
    /// Pushed in proportion to its weight.
    #[default]
    Free,
    /// Never pushed: actors that bump it take the whole push.
    Immovable,
}

/// How far apart two actors' capsules may come before they push each other apart, in m (R23);
/// negative, they overlap by that much first. A capsule is the actor's personal space, wider than
/// its body, so even overlapping capsules leave a pushed actor seeming to step aside rather than
/// be struck, and the player can walk up close to an actor to talk without shoving it.
const AURA_GAP: f32 = -0.1;

/// The speed two actors separate at, in m/s, per metre they are past `AURA_GAP`. An actor walking
/// into another settles about its speed over this further in.
const STIFFNESS: f32 = 20.0;

/// The time constant a push eases toward its target with, in s.
const PUSH_SMOOTHING: f32 = 0.1;

/// The fastest an actor is pushed, in m/s (R28).
const MAX_PUSH_SPEED: f32 = 4.0;

/// A push slower than this with nothing pushing ends, in m/s.
const PUSH_EPSILON: f32 = 0.01;

/// How much an actor resists being pushed: its capsule's volume, or infinite if it is immovable.
pub fn actor_weight(
    size: Option<&CollisionSize>,
    scale: Option<&ActorScale>,
    footing: ActorFooting,
) -> f32 {
    let shape = capsule(size, scale);
    let radius = shape.radius;
    match footing {
        ActorFooting::Free => {
            PI * radius * radius * (2.0 * shape.half_height() + 4.0 / 3.0 * radius)
        }
        ActorFooting::Immovable => f32::INFINITY,
    }
}

/// An actor, as the bumping sees it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Body {
    entity: Entity,
    /// The feet.
    position: Vec3,
    radius: f32,
    height: f32,
    weight: f32,
    resting: bool,
}

/// The shares of a separation two actors of weights `a` and `b` take: each the other's weight's
/// part of the whole, so the lighter moves more, and an immovable actor not at all.
fn shares(a: f32, b: f32) -> (f32, f32) {
    match (a.is_infinite(), b.is_infinite()) {
        (true, true) => (0.0, 0.0),
        (true, false) => (0.0, 1.0),
        (false, true) => (1.0, 0.0),
        (false, false) => (b / (a + b), a / (a + b)),
    }
}

/// The pushes two actors give each other, on the ground plane, if they are within each other's
/// aura and at overlapping heights. Two on the same spot part along x.
fn pair_push(a: &Body, b: &Body) -> Option<(Vec2, Vec2)> {
    let overlap_y =
        a.position.y < b.position.y + b.height && b.position.y < a.position.y + a.height;
    if !overlap_y {
        return None;
    }
    let offset = (b.position - a.position).xz();
    let distance = offset.length();
    let shortfall = AURA_GAP - (distance - a.radius - b.radius);
    if shortfall <= 0.0 {
        return None;
    }
    let apart = if distance > 1e-4 {
        offset / distance
    } else {
        Vec2::X
    };
    let speed = STIFFNESS * shortfall;
    let (share_a, share_b) = shares(a.weight, b.weight);
    Some((-apart * speed * share_a, apart * speed * share_b))
}

/// Eases `push` toward `target` over a tick of `dt`, capped at `MAX_PUSH_SPEED`; zero once it has
/// all but stopped with nothing pushing.
fn ease(push: Vec2, target: Vec2, dt: f32) -> Vec2 {
    let eased = push + (target - push) * (1.0 - (-dt / PUSH_SMOOTHING).exp());
    let eased = eased.clamp_length_max(MAX_PUSH_SPEED);
    if target == Vec2::ZERO && eased.length() < PUSH_EPSILON {
        Vec2::ZERO
    } else {
        eased
    }
}

/// Sets each actor's push (`ActorMotion::push`) in the viewpoint's realm, the only one that
/// moves, from every pair of actors there. Two resting actors cannot have come nearer, so their
/// pair is skipped. Runs before `move_actors`, which wakes a resting actor that is pushed.
pub(crate) fn bump_actors(
    mut actors: Query<
        (
            Entity,
            &Actor,
            &mut ActorMotion,
            Option<&CollisionSize>,
            Option<&ActorScale>,
            Option<&ActorFooting>,
            Has<ActorResting>,
        ),
        Without<ActorHeld>,
    >,
    viewpoint: Res<Viewpoint>,
    time: Res<Time<Fixed>>,
    mut bodies: Local<Vec<Body>>,
    mut targets: Local<Vec<Vec2>>,
) {
    let Some(realm) = viewpoint.realm else {
        return;
    };
    let dt = time.timestep().as_secs_f32();
    bodies.clear();
    for (entity, actor, motion, size, scale, footing, resting) in &actors {
        if actor.realm != realm {
            continue;
        }
        let shape = capsule(size, scale);
        bodies.push(Body {
            entity,
            position: motion.position,
            radius: shape.radius,
            height: 2.0 * (shape.half_height() + shape.radius),
            weight: actor_weight(size, scale, footing.copied().unwrap_or_default()),
            resting,
        });
    }
    targets.clear();
    targets.resize(bodies.len(), Vec2::ZERO);
    for i in 0..bodies.len() {
        for j in i + 1..bodies.len() {
            let (a, b) = (&bodies[i], &bodies[j]);
            if a.resting && b.resting {
                continue;
            }
            if let Some((push_a, push_b)) = pair_push(a, b) {
                targets[i] += push_a;
                targets[j] += push_b;
            }
        }
    }
    for (body, &target) in bodies.iter().zip(targets.iter()) {
        let Ok((_, _, mut motion, ..)) = actors.get_mut(body.entity) else {
            continue;
        };
        let push = ease(motion.push, target, dt);
        if motion.push != push {
            motion.push = push;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 64.0;

    /// Apart along x by this, two `body`s are 0.05 m inside each other's aura.
    const INSIDE: f32 = 1.0 + AURA_GAP - 0.05;

    /// Apart along x by this, two `body`s are 0.1 m outside each other's aura.
    const OUTSIDE: f32 = 1.0 + AURA_GAP + 0.1;

    fn body(x: f32, y: f32, weight: f32) -> Body {
        Body {
            entity: Entity::PLACEHOLDER,
            position: Vec3::new(x, y, 0.0),
            radius: 0.5,
            height: 1.8,
            weight,
            resting: false,
        }
    }

    #[test]
    fn equal_weights_share_the_push() {
        let (a, b) = pair_push(&body(0.0, 0.0, 1.0), &body(INSIDE, 0.0, 1.0)).unwrap();
        assert!(a.x < 0.0 && b.x > 0.0, "{a} {b}");
        assert!((a.length() - b.length()).abs() < 1e-5, "{a} {b}");
    }

    #[test]
    fn an_immovable_actor_takes_none_of_the_push() {
        let (a, b) = pair_push(&body(0.0, 0.0, f32::INFINITY), &body(INSIDE, 0.0, 1.0)).unwrap();
        assert_eq!(a, Vec2::ZERO);
        assert!(b.x > 0.0);
        let speed = STIFFNESS * 0.05;
        assert!((b.length() - speed).abs() < 1e-4, "{b}");
    }

    #[test]
    fn the_lighter_actor_moves_more() {
        let (a, b) = pair_push(&body(0.0, 0.0, 2.0), &body(INSIDE, 0.0, 1.0)).unwrap();
        assert!((b.length() - 2.0 * a.length()).abs() < 1e-5, "{a} {b}");
    }

    #[test]
    fn actors_apart_or_at_different_heights_do_not_push() {
        assert!(pair_push(&body(0.0, 0.0, 1.0), &body(OUTSIDE, 0.0, 1.0)).is_none());
        assert!(pair_push(&body(0.0, 0.0, 1.0), &body(0.5, 2.0, 1.0)).is_none());
    }

    #[test]
    fn a_push_is_capped_and_dies_away() {
        let mut push = Vec2::ZERO;
        for _ in 0..64 {
            push = ease(push, Vec2::X * 100.0, DT);
        }
        assert!((push.length() - MAX_PUSH_SPEED).abs() < 1e-4, "{push}");
        for _ in 0..64 {
            push = ease(push, Vec2::ZERO, DT);
        }
        assert_eq!(push, Vec2::ZERO);
    }
}
