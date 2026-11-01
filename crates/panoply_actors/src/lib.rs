use bevy::{app::Propagate, prelude::*};
use panoply_core::Realm;
use panoply_exemplar::{Exemplar, ExemplarServer, InstanceType, UpdateAspects};

mod actor_animation;
mod actor_aspect;
mod actor_model;

pub use actor_animation::ActorArmature;
pub use actor_model::*;

use self::actor_animation::{bind_actor_skeleton, play_actor_idle};

use self::actor_aspect::{Armature, ColorSlots, Colors, FeatureSlots, Features, Skin};

pub const ACTOR_TYPE: InstanceType = InstanceType::from_str("Actr");

/// A character: the player, an NPC or a monster.
#[derive(Debug, Component)]
pub struct Actor {
    /// The actor's archetype.
    pub exemplar: Handle<Exemplar>,
}

/// Marks an actor whose aspects need applying from its exemplar.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ActorRebuildAspects;

/// Marks an actor whose body needs spawning, after its aspects changed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ActorRebuildModels;

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Armature>()
            .register_type::<Skin>()
            .register_type::<ColorSlots>()
            .register_type::<Colors>()
            .register_type::<FeatureSlots>()
            .register_type::<Features>()
            .add_systems(
                Update,
                (
                    spawn_player,
                    update_actor_aspects,
                    spawn_actor_bodies,
                    spawn_actor_body_scenes,
                    play_actor_idle,
                )
                    .chain(),
            )
            .add_observer(bind_actor_skeleton);
    }
}

/// TODO(PAN3l): the player comes from a precinct. Until then, one actor stands on a floor in the
/// playground (`plans/physics.py` measures its top at 0.02).
const PLAYER_EXEMPLAR: &str = "exemplars/characters.exem.json#Farmer1";
const PLAYER_REALM: &str = "playground";
const PLAYER_POSITION: Vec3 = Vec3::new(58.0, 0.02, 26.62);

fn spawn_player(
    mut commands: Commands,
    mut exemplars: ExemplarServer,
    realms: Query<&Realm>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(realm) = realms.iter().find(|realm| realm.name == PLAYER_REALM) else {
        return;
    };
    let Some(exemplar) = exemplars.get(PLAYER_EXEMPLAR) else {
        return;
    };
    commands.spawn((
        Name::new("Actor"),
        Actor { exemplar },
        ActorRebuildAspects,
        Transform::from_translation(PLAYER_POSITION),
        Visibility::default(),
        Propagate(realm.layer.clone()),
    ));
    *spawned = true;
}

fn update_actor_aspects(
    mut commands: Commands,
    query: Query<(Entity, &Actor), With<ActorRebuildAspects>>,
    server: Res<AssetServer>,
) {
    for (entity, actor) in query.iter() {
        if server.load_state(&actor.exemplar).is_loaded() {
            commands
                .entity(entity)
                .queue(UpdateAspects {
                    exemplar: actor.exemplar.clone(),
                    finish: ActorRebuildModels,
                })
                .remove::<ActorRebuildAspects>();
        }
    }
}
