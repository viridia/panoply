use bevy::{gltf::Gltf, prelude::*, world_serialization::WorldAssetRoot};

use super::{actor_aspect::Skin, ActorRebuildModels};

/// An actor's body, a child of the actor: one named scene of a character glTF, holding the body
/// mesh and its skeleton (D28). Waits for the file to load, then spawns the scene.
#[derive(Debug, Component)]
pub struct ActorBody {
    gltf: Handle<Gltf>,
    scene: String,
}

/// Replaces the body of each actor whose aspects changed. The root `Gltf` is loaded, not the
/// scene by label, which would load the whole file again for each actor (D28).
pub fn spawn_actor_bodies(
    mut commands: Commands,
    query: Query<(Entity, Option<&Skin>), With<ActorRebuildModels>>,
    server: Res<AssetServer>,
) {
    for (entity, skin) in query.iter() {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .remove::<ActorRebuildModels>();
        let Some((file, scene)) = skin.and_then(|skin| skin.0.split_once('#')) else {
            warn!("Actor {entity} has no skin");
            continue;
        };
        commands.spawn((
            Name::new("ActorBody"),
            ActorBody {
                gltf: server.load(file.to_owned()),
                scene: scene.to_owned(),
            },
            ChildOf(entity),
        ));
    }
}

/// Spawns each body's scene once its file has loaded.
pub fn spawn_actor_body_scenes(
    mut commands: Commands,
    query: Query<(Entity, &ActorBody), Without<WorldAssetRoot>>,
    gltfs: Res<Assets<Gltf>>,
) {
    for (entity, body) in query.iter() {
        let Some(gltf) = gltfs.get(&body.gltf) else {
            continue;
        };
        if let Some(scene) = gltf.named_scenes.get(body.scene.as_str()) {
            commands.entity(entity).insert(WorldAssetRoot(scene.clone()));
        } else {
            error!(
                "Actor body [{}] not found; scenes: {:?}",
                body.scene,
                gltf.named_scenes.keys()
            );
            commands.entity(entity).despawn();
        }
    }
}
