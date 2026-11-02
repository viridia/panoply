use bevy::{
    gltf::Gltf,
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstanceReady},
};
use panoply_exemplar::{Exemplar, ExemplarServer};

use super::{
    actor_aspect::WieldedItems,
    item_aspect::{EquipSlot, ItemModel},
    ActorBody,
};

/// An item the actor holds: its model's scene, a child of the bone its `EquipSlot` names (R14).
/// Starts as a child of the actor, and moves to the bone once its exemplar and model have loaded.
#[derive(Debug, Component)]
pub struct WieldedItem {
    /// The item's exemplar path.
    item: String,
    /// The item's model and slot, once its exemplar has loaded.
    model: Option<WieldedModel>,
}

#[derive(Debug)]
struct WieldedModel {
    gltf: Handle<Gltf>,
    scene: String,
    slot: String,
}

/// Spawns the actor's wielded items once its body has spawned, since they attach to its bones.
pub fn spawn_wielded_items(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    bodies: Query<&ChildOf, With<ActorBody>>,
    wielded: Query<&WieldedItems>,
) {
    let Ok(actor) = bodies.get(ready.entity) else {
        return;
    };
    let Ok(wielded) = wielded.get(actor.parent()) else {
        return;
    };
    for item in wielded.0.iter() {
        commands.spawn((
            Name::new("WieldedItem"),
            WieldedItem {
                item: item.clone(),
                model: None,
            },
            ChildOf(actor.parent()),
        ));
    }
}

/// Attaches each wielded item's model to its slot's bone, once the item's exemplar and the
/// model's file have loaded. The root `Gltf` is loaded, not the scene by label (D28).
pub fn attach_wielded_items(
    mut commands: Commands,
    mut query: Query<(Entity, &mut WieldedItem, &ChildOf), Without<WorldAssetRoot>>,
    mut exemplar_server: ExemplarServer,
    exemplars: Res<Assets<Exemplar>>,
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    actors: Query<&Children>,
    bodies: Query<(), With<ActorBody>>,
    names: Query<&Name>,
) {
    for (entity, mut wielded, actor) in query.iter_mut() {
        if wielded.model.is_none() {
            let Some(exemplar) = exemplar_server.get(&wielded.item) else {
                continue;
            };
            if !exemplars.contains(&exemplar) {
                continue;
            }
            let model = Exemplar::find_aspect::<ItemModel>(&exemplars, &exemplar);
            let slot = Exemplar::find_aspect::<EquipSlot>(&exemplars, &exemplar);
            let Some(((file, scene), slot)) = model.and_then(|m| m.0.split_once('#')).zip(slot)
            else {
                error!("Wielded item [{}] has no model or slot", wielded.item);
                commands.entity(entity).despawn();
                continue;
            };
            wielded.model = Some(WieldedModel {
                gltf: server.load(file.to_owned()),
                scene: scene.to_owned(),
                slot: slot.0.clone(),
            });
        }
        let model = wielded.model.as_ref().unwrap();
        let Some(gltf) = gltfs.get(&model.gltf) else {
            continue;
        };
        let Some(scene) = gltf.named_scenes.get(model.scene.as_str()) else {
            error!("Wielded item model [{}] not found", model.scene);
            commands.entity(entity).despawn();
            continue;
        };
        let bone = actors
            .get(actor.parent())
            .into_iter()
            .flatten()
            .filter(|child| bodies.contains(**child))
            .flat_map(|body| actors.iter_descendants(*body))
            .find(|entity| names.get(*entity).is_ok_and(|name| name.as_str() == model.slot));
        let Some(bone) = bone else {
            error!("Wielded item bone [{}] not found in the body", model.slot);
            commands.entity(entity).despawn();
            continue;
        };
        commands
            .entity(entity)
            .insert((WorldAssetRoot(scene.clone()), ChildOf(bone)));
    }
}
