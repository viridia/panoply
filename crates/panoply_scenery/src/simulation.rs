//! The simulated precincts (R8): those in the viewpoint's realm that the simulation box overlaps
//! ([`Viewpoint::simulated_precincts`]). Each needs a navigation mesh, and NPCs run their own
//! logic there.

use bevy::prelude::*;
use panoply_core::Viewpoint;

use crate::precinct::Precinct;

/// Marks a precinct the simulation box overlaps.
#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct SimulatedPrecinct;

/// Keeps [`SimulatedPrecinct`] on the precincts the box overlaps, and off the rest.
pub(crate) fn mark_simulated_precincts(
    viewpoint: Res<Viewpoint>,
    precincts: Query<(Entity, &Precinct, Has<SimulatedPrecinct>)>,
    mut commands: Commands,
) {
    let simulated = viewpoint.simulated_precincts();
    for (entity, precinct, marked) in &precincts {
        let overlaps = viewpoint.realm == Some(precinct.realm)
            && (simulated.min.x..simulated.max.x).contains(&precinct.coords.x)
            && (simulated.min.y..simulated.max.y).contains(&precinct.coords.y);
        if overlaps && !marked {
            commands.entity(entity).insert(SimulatedPrecinct);
        } else if !overlaps && marked {
            commands.entity(entity).remove::<SimulatedPrecinct>();
        }
    }
}
