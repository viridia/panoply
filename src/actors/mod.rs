use bevy::app::{App, Plugin};
use panoply_exemplar::InstanceType;

mod actor_aspect;
mod actor_instance;

pub use actor_instance::*;

use self::actor_aspect::{Armature, ColorSlots, Colors, Combatant, FeatureSlots, Features, Skin};

pub const ACTOR_TYPE: InstanceType = InstanceType::from_str("Actr");

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Armature>()
            .register_type::<Skin>()
            .register_type::<ColorSlots>()
            .register_type::<Colors>()
            .register_type::<FeatureSlots>()
            .register_type::<Features>()
            .register_type::<Combatant>();
    }
}
