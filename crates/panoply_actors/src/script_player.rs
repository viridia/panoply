//! `player` in formulas: the entity of the actor the player controls.

use bevy::prelude::*;
use panoply_saga::{ExprType, SagaHostBuilder, ScriptVm, StackValue, VMError};

use crate::PlayerActor;

/// The actor the player controls, as formulas read it. A resource, so that a formula reading
/// `player` runs again when the player's actor arrives or goes.
#[derive(Resource, Default)]
pub struct ScriptPlayer(pub Option<Entity>);

/// Registers `player`. Call it while plugins build, after `SagaPlugin`.
pub(crate) fn add_script_player(app: &mut App) {
    app.init_resource::<ScriptPlayer>()
        .add_observer(
            |add: On<Add<PlayerActor>>, mut player: ResMut<ScriptPlayer>| {
                player.0 = Some(add.entity);
            },
        )
        .add_observer(
            |remove: On<Remove<PlayerActor>>, mut player: ResMut<ScriptPlayer>| {
                if player.0 == Some(remove.entity) {
                    player.0 = None;
                }
            },
        );
    app.world_mut()
        .get_resource_mut::<SagaHostBuilder>()
        .expect("ActorsPlugin builds after SagaPlugin")
        .0
        .add_global_prop("player", read_player, ExprType::Entity);
}

/// The player's actor; while there is none, an entity that has nothing, so that whatever is read
/// from it is absent.
fn read_player(vm: &mut ScriptVm) -> Result<StackValue, VMError> {
    vm.tracking.track_resource::<ScriptPlayer>(vm.world);
    let player = vm
        .world
        .get_resource::<ScriptPlayer>()
        .and_then(|player| player.0);
    Ok(StackValue::Entity(player.unwrap_or(Entity::PLACEHOLDER)))
}
