use bevy::app::{App, Plugin};
use panoply_exemplar::InstanceType;

mod actor_aspect;
mod actor_instance;

pub use actor_instance::*;

use self::actor_aspect::Combatant;

pub const ACTOR_TYPE: InstanceType = InstanceType::from_str("Actr");

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Combatant>();
    }
}
