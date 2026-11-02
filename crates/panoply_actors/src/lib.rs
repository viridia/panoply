use bevy::{app::Propagate, prelude::*};
use panoply_core::{realm_physics::realm_physics_system, Realm, Viewpoint};
use panoply_exemplar::{Exemplar, ExemplarServer, InstanceAspects, InstanceType, UpdateAspects};

mod actor_animation;
mod actor_aspect;
mod actor_colors;
mod actor_control;
mod actor_features;
mod actor_model;
mod actor_physics;
mod actor_travel;
mod actor_wield;
mod item_aspect;
mod precinct_actors;

pub use actor_animation::ActorArmature;
pub use actor_aspect::{ActorScale, CollisionSize};
pub use actor_control::{ActorIntent, PlayerActor};
pub use actor_features::ActorFeature;
pub use actor_model::*;
pub use actor_physics::ActorMotion;
pub use actor_travel::{ActionFinished, TravelTo};
pub use actor_wield::WieldedItem;
pub use precinct_actors::{HomePrecinct, PrecinctActors};

use self::actor_animation::{animate_actor_gait, bind_actor_skeleton, build_actor_graph};
use self::actor_colors::{color_actor_part, ActorMaterials};
use self::actor_control::drive_player;
use self::actor_features::{bind_actor_feature, spawn_actor_features};
use self::actor_physics::{interpolate_actors, move_actors, start_actor_physics};
use self::actor_travel::travel_actors;
use self::actor_wield::{attach_wielded_items, spawn_wielded_items};
use self::precinct_actors::{mark_precinct_actors, spawn_precinct_actors};

use self::actor_aspect::{
    Armature, ClipNames, ColorSlots, Colors, FeatureSlots, Features, Skin, WieldedItems,
};
use self::item_aspect::{EquipSlot, ItemModel};

pub const ACTOR_TYPE: InstanceType = InstanceType::from_str("Actr");
pub const ITEM_TYPE: InstanceType = InstanceType::from_str("Item");

/// A character: the player, an NPC or a monster.
#[derive(Debug, Component)]
pub struct Actor {
    /// The actor's archetype.
    pub exemplar: Handle<Exemplar>,
    /// The realm the actor is in.
    pub realm: Entity,
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
            .register_type::<ClipNames>()
            .register_type::<Skin>()
            .register_type::<CollisionSize>()
            .register_type::<ActorScale>()
            .register_type::<HomePrecinct>()
            .register_type::<PrecinctActors>()
            .register_type::<ActorMotion>()
            .register_type::<ActorIntent>()
            .register_type::<TravelTo>()
            .register_type::<ColorSlots>()
            .register_type::<Colors>()
            .register_type::<FeatureSlots>()
            .register_type::<Features>()
            .register_type::<WieldedItems>()
            .register_type::<ItemModel>()
            .register_type::<EquipSlot>()
            .add_systems(
                Update,
                (
                    spawn_player,
                    mark_precinct_actors,
                    spawn_precinct_actors,
                    update_actor_aspects,
                    spawn_actor_bodies,
                    spawn_actor_body_scenes,
                    build_actor_graph,
                    attach_wielded_items,
                    start_actor_physics,
                    drive_player,
                    animate_actor_gait,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (travel_actors, move_actors)
                    .chain()
                    .after(realm_physics_system),
            )
            .add_systems(
                RunFixedMainLoop,
                interpolate_actors.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            )
            .init_resource::<ActorMaterials>()
            .add_observer(bind_actor_skeleton)
            .add_observer(spawn_actor_features)
            .add_observer(bind_actor_feature)
            .add_observer(spawn_wielded_items)
            .add_observer(color_actor_part);
    }
}

/// TODO(X46): a new game places the player. Until then, the player's actor drops from 2 m over
/// the playground's ground level, near the table that `plans/physics.py` measures on a floor at
/// 0, and the camera follows it.
const PLAYER_EXEMPLAR: &str = "exemplars/characters.exem.json#Farmer1";
const PLAYER_REALM: &str = "playground";
const PLAYER_POSITION: Vec3 = Vec3::new(37.0, 2.0, 47.0);
const PLAYER_WIELDS: &str = "exemplars/weapons.exem.json#Broadsword";

fn spawn_player(
    mut commands: Commands,
    mut exemplars: ExemplarServer,
    mut viewpoint: ResMut<Viewpoint>,
    realms: Query<(Entity, &Realm)>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some((realm_entity, realm)) = realms.iter().find(|(_, realm)| realm.name == PLAYER_REALM)
    else {
        return;
    };
    let Some(exemplar) = exemplars.get(PLAYER_EXEMPLAR) else {
        return;
    };
    let actor = commands.spawn((
        Name::new("Actor"),
        Actor {
            exemplar,
            realm: realm_entity,
        },
        PlayerActor,
        ActorIntent::default(),
        InstanceAspects(vec![Box::new(WieldedItems(vec![PLAYER_WIELDS.to_owned()]))]),
        ActorRebuildAspects,
        Transform::from_translation(PLAYER_POSITION),
        Visibility::default(),
        Propagate(realm.layer.clone()),
    ));
    viewpoint.follow = Some(actor.id());
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
