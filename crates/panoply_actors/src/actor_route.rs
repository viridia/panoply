//! Travel by route (R43): an actor asked to go somewhere is given a route there over the
//! navigation mesh and travels its turns. The player's click asks for one (R92), as NPCs' behaviors
//! will. An actor that cannot advance replans (R22).

use bevy::prelude::*;
use panoply_nav::{NavCosts, NavRoutes, PlannedRoute, RouteError, RouteQuery};

use crate::{ActionFinished, Actor, ActorIntent, ActorMotion, TravelTo};

/// How long an actor stays blocked before it replans, in seconds (R22).
const REPLAN_DELAY: f32 = 1.0;

/// How many times a blocked actor replans before it gives up, as faery's planner did.
const MAX_REPLANS: u32 = 3;

/// Travel to `goal`, or to within `reach` of it, at `speed`, in m/s, by a route over the
/// navigation mesh. While it has no [`PlannedRoute`], one is asked for each tick until the meshes
/// it needs are built; then the actor travels the route's turns. Removed when the actor arrives,
/// which triggers [`ActionFinished`], when it gives up, or when something pre-empts it (R45),
/// which also takes away its route and its travel.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct RouteTo {
    pub goal: Vec3,
    pub reach: f32,
    pub speed: f32,
    /// How many times it has replanned, blocked.
    pub replans: u32,
}

impl RouteTo {
    pub fn new(goal: Vec3, reach: f32, speed: f32) -> Self {
        Self {
            goal,
            reach,
            speed,
            replans: 0,
        }
    }
}

/// A new request replaces the route and the travel of the one before.
pub(crate) fn replace_route(insert: On<Insert<RouteTo>>, mut commands: Commands) {
    commands
        .entity(insert.entity)
        .remove::<(PlannedRoute, TravelTo)>();
}

/// A request removed takes its route and travel with it.
pub(crate) fn end_route(remove: On<Remove<RouteTo>>, mut commands: Commands) {
    commands
        .entity(remove.entity)
        .try_remove::<(PlannedRoute, TravelTo)>();
}

/// Travel's end is the request's.
pub(crate) fn finish_route(
    finished: On<ActionFinished>,
    requests: Query<(), With<RouteTo>>,
    mut commands: Commands,
) {
    if requests.contains(finished.entity) {
        commands.entity(finished.entity).remove::<RouteTo>();
    }
}

/// Asks a route for each request without one. A request waits while a mesh it needs is not
/// built; one whose actor stands off the mesh is dropped.
pub(crate) fn plan_routes(
    mut commands: Commands,
    actors: Query<(Entity, &Actor, &ActorMotion, &RouteTo), Without<PlannedRoute>>,
    routes: NavRoutes,
) {
    for (entity, actor, motion, request) in &actors {
        let query = RouteQuery {
            start: motion.position,
            goal: request.goal,
            reach: request.reach,
            costs: NavCosts::default(),
        };
        match routes.route(actor.realm, &query) {
            Ok(route) => {
                let travel = TravelTo::through(route.turns(), request.speed);
                commands
                    .entity(entity)
                    .insert((travel, PlannedRoute(route)));
            }
            Err(RouteError::NotReady(_)) => {}
            Err(error) => {
                warn!("No route to {}: {error}", request.goal);
                commands.entity(entity).remove::<RouteTo>();
            }
        }
    }
}

/// An actor blocked for [`REPLAN_DELAY`] on its route stops. Within reach of its goal, it has
/// arrived; otherwise it replans, until it has done so [`MAX_REPLANS`] times, then gives up.
pub(crate) fn replan_blocked(
    mut commands: Commands,
    mut actors: Query<(Entity, &ActorMotion, &mut RouteTo, &mut ActorIntent), With<PlannedRoute>>,
    time: Res<Time<Fixed>>,
) {
    let delay = (REPLAN_DELAY / time.timestep().as_secs_f32()).ceil() as u32;
    for (entity, motion, mut request, mut intent) in &mut actors {
        if motion.blocked_ticks() < delay {
            continue;
        }
        *intent = ActorIntent {
            heading: motion.facing,
            speed: 0.0,
        };
        let distance = (request.goal - motion.position).xz().length();
        if distance <= request.reach {
            commands.entity(entity).remove::<TravelTo>();
            commands.trigger(ActionFinished { entity });
        } else if request.replans < MAX_REPLANS {
            request.replans += 1;
            commands.entity(entity).remove::<(PlannedRoute, TravelTo)>();
        } else {
            warn!("Blocked on the way to {}; giving up", request.goal);
            commands.entity(entity).remove::<RouteTo>();
        }
    }
}
