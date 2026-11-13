use bevy::{
    gltf::Gltf,
    mesh::skinning::SkinnedMesh,
    platform::collections::HashMap,
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstanceReady},
};

use super::{actor_aspect::Features, ActorBody};

/// A feature of an actor (hair, a hat, a beard), a child of the actor: a named scene of the
/// body's character file, whose meshes are skinned to the body's skeleton (D28).
#[derive(Debug, Component)]
pub struct ActorFeature;

/// Spawns the actor's enabled features once its body has spawned, since they bind to its bones.
pub fn spawn_actor_features(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    bodies: Query<(&ActorBody, &ChildOf)>,
    features: Query<&Features>,
    gltfs: Res<Assets<Gltf>>,
) {
    let Ok((body, actor)) = bodies.get(ready.entity) else {
        return;
    };
    let Ok(features) = features.get(actor.parent()) else {
        return;
    };
    let Some(gltf) = gltfs.get(&body.gltf) else {
        return;
    };
    for (feature, _) in features.0.iter().filter(|(_, enabled)| **enabled) {
        let Some(scene) = gltf.named_scenes.get(feature.as_str()) else {
            let mut scenes: Vec<_> = gltf.named_scenes.keys().collect();
            scenes.sort();
            error!("Actor feature [{feature}] not found; scenes: {scenes:?}");
            continue;
        };
        commands.spawn((
            Name::new("ActorFeature"),
            ActorFeature,
            WorldAssetRoot(scene.clone()),
            ChildOf(actor.parent()),
        ));
    }
}

/// Binds a feature to the body's skeleton once its scene has spawned. Each feature scene carries
/// its own copy of the armature (the export adds it to every skinned scene): its meshes' joints
/// are pointed at the body's bones of the same name, and the copy is despawned, so the actor has
/// one skeleton to animate. Bevy has no way yet to spawn a skinned mesh against another scene's
/// skeleton (bevyengine/bevy#17114).
pub fn bind_actor_feature(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    features: Query<&ChildOf, With<ActorFeature>>,
    actors: Query<&Children>,
    bodies: Query<&ActorBody>,
    skins: Query<&SkinnedMesh>,
    names: Query<&Name>,
) {
    let Ok(actor) = features.get(ready.entity) else {
        return;
    };
    let Some((body_entity, body)) = actors
        .get(actor.parent())
        .into_iter()
        .flatten()
        .find_map(|child| Some((*child, bodies.get(*child).ok()?)))
    else {
        return;
    };
    let Some(node) = body.armature.as_ref().and_then(|armature| armature.node.as_ref()) else {
        return;
    };
    let bones: HashMap<&str, Entity> = actors
        .iter_descendants(body_entity)
        .filter_map(|entity| Some((names.get(entity).ok()?.as_str(), entity)))
        .collect();
    for entity in actors.iter_descendants(ready.entity) {
        let Ok(skin) = skins.get(entity) else {
            if names.get(entity).is_ok_and(|name| name.as_str() == node) {
                commands.entity(entity).despawn();
            }
            continue;
        };
        let joints = skin
            .joints
            .iter()
            .map(|joint| {
                let name = names.get(*joint).map_or("", Name::as_str);
                bones.get(name).copied().ok_or(name)
            })
            .collect::<Result<Vec<_>, _>>();
        match joints {
            Ok(joints) => {
                commands.entity(entity).insert(SkinnedMesh {
                    inverse_bindposes: skin.inverse_bindposes.clone(),
                    joints,
                });
            }
            Err(name) => error!("Actor feature bone [{name}] not found in the body"),
        }
    }
}
