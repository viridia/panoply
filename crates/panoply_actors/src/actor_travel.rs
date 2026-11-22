//! Body actions that move an actor (R43). Travel takes an actor through waypoints in straight
//! lines at a pace, through the character controller, so it is stopped by what is in the way.

use bevy::prelude::*;

use crate::{actor_control::angle_between, ActorIntent, ActorMotion};

/// How near a waypoint, across the ground, counts as there, in m.
const ARRIVAL_DISTANCE: f32 = 0.05;

/// How far from the next leg an actor that has overshot a waypoint may be, across the ground, and
/// go on from where it is rather than turn back, in m.
const LEG_TOLERANCE: f32 = 0.2;

/// Travel through `waypoints` in turn at `speed`, in m/s. Removed when the actor arrives at the
/// last, which triggers `ActionFinished`, or when something pre-empts it, which does not (R45).
/// Height is ignored: the actor stays on the ground.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct TravelTo {
    pub waypoints: Vec<Vec3>,
    pub speed: f32,
    /// Where the leg to the first waypoint began: the waypoint before it, or where the actor stood
    /// when it set out. Set by travel.
    #[reflect(default)]
    pub from: Option<Vec3>,
}

impl TravelTo {
    /// Travel through `waypoints`.
    pub fn through(waypoints: Vec<Vec3>, speed: f32) -> Self {
        Self {
            waypoints,
            speed,
            from: None,
        }
    }

    /// Travel to one point.
    pub fn to(target: Vec3, speed: f32) -> Self {
        Self::through(vec![target], speed)
    }
}

/// Whether an actor at `at`, on its way from `from` to `waypoint` and on to `next`, is done with
/// `waypoint`: on it, or past it and near enough the next leg to go on from where it is. Seen from
/// above.
fn passed(at: Vec2, from: Vec2, waypoint: Vec2, next: Vec2) -> bool {
    if at.distance(waypoint) < ARRIVAL_DISTANCE {
        return true;
    }
    let past = (at - waypoint).dot(waypoint - from) > 0.0;
    let leg = next - waypoint;
    let t = ((at - waypoint).dot(leg) / leg.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
    past && at.distance(waypoint + leg * t) <= LEG_TOLERANCE
}

/// Triggered on an actor when its body action finishes (R45).
#[derive(EntityEvent, Debug)]
pub struct ActionFinished {
    pub entity: Entity,
}

/// Steers travelling actors toward their next waypoints, one fixed tick at a time, before they
/// move.
pub(crate) fn travel_actors(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut TravelTo, &ActorMotion, &mut ActorIntent)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.timestep().as_secs_f32();
    for (entity, mut travel, motion, mut intent) in &mut actors {
        let at = motion.position.xz();
        let near = |point: &Vec3| (*point - motion.position).xz().length() < ARRIVAL_DISTANCE;
        let mut from = *travel.from.get_or_insert(motion.position);
        while travel.waypoints.len() > 1
            && passed(
                at,
                from.xz(),
                travel.waypoints[0].xz(),
                travel.waypoints[1].xz(),
            )
        {
            from = travel.waypoints.remove(0);
            travel.from = Some(from);
        }
        let Some(&target) = travel.waypoints.first().filter(|point| !near(point)) else {
            *intent = ActorIntent {
                heading: motion.facing,
                speed: 0.0,
            };
            commands.entity(entity).remove::<TravelTo>();
            commands.trigger(ActionFinished { entity });
            continue;
        };
        let offset = (target - motion.position).xz();
        let distance = offset.length();
        let heading = offset.x.atan2(offset.y);
        // Slowed while turning, so the actor turns toward a near target rather than circling it;
        // further off, `move_actors` turns it in place. Never past the target in one tick.
        let alignment = angle_between(motion.facing, heading).cos().max(0.0);
        let speed = (travel.speed * alignment).min(distance / dt);
        *intent = ActorIntent { heading, speed };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A right-angle corner: east from (0, 0) to (2, 0), then north to (2, 2).
    const FROM: Vec2 = Vec2::new(0.0, 0.0);
    const CORNER: Vec2 = Vec2::new(2.0, 0.0);
    const NEXT: Vec2 = Vec2::new(2.0, 2.0);

    #[test]
    fn on_the_waypoint_is_passed() {
        assert!(passed(Vec2::new(1.97, 0.02), FROM, CORNER, NEXT));
    }

    #[test]
    fn short_of_the_waypoint_is_not() {
        assert!(!passed(Vec2::new(1.9, 0.0), FROM, CORNER, NEXT));
    }

    #[test]
    fn a_slight_overshoot_goes_on() {
        assert!(passed(Vec2::new(2.1, 0.05), FROM, CORNER, NEXT));
    }

    #[test]
    fn an_overshoot_far_from_the_next_leg_turns_back() {
        assert!(!passed(Vec2::new(2.5, 0.0), FROM, CORNER, NEXT));
    }

    #[test]
    fn beside_the_next_leg_but_short_of_the_waypoint_is_not() {
        // Within the tolerance of the next leg's start, but not yet past the corner.
        assert!(!passed(Vec2::new(1.85, 0.1), FROM, CORNER, NEXT));
    }
}
