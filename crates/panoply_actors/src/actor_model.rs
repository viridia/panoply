use bevy::{gltf::Gltf, prelude::*, world_serialization::WorldAssetRoot};

use super::{
    actor_animation::ArmatureSource,
    actor_aspect::{Armature, ClipNames, Skin},
    ActorRebuildModels,
};

/// An actor's body, a child of the actor: one named scene of a character glTF, holding the body
/// mesh and its skeleton (D28). Waits for the file to load, then spawns the scene.
#[derive(Debug, Component)]
pub struct ActorBody {
    /// The character file, which also holds the features (D28).
    pub(crate) gltf: Handle<Gltf>,
    scene: String,
    /// The skeleton to animate; `None` for an actor without an `Armature`.
    pub(crate) armature: Option<ArmatureSource>,
}

/// Replaces the body of each actor whose aspects changed. The root `Gltf` is loaded, not the
/// scene by label, which would load the whole file again for each actor (D28).
pub fn spawn_actor_bodies(
    mut commands: Commands,
    query: Query<
        (Entity, Option<&Skin>, Option<&Armature>, Option<&ClipNames>),
        With<ActorRebuildModels>,
    >,
    server: Res<AssetServer>,
) {
    for (entity, skin, armature, clip_names) in query.iter() {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .remove::<ActorRebuildModels>();
        let Some((file, scene)) = skin.and_then(|skin| skin.0.split_once('#')) else {
            warn!("Actor {entity} has no skin");
            continue;
        };
        // Clips come from the armature's own file when the species has no shared one.
        let armature = armature.and_then(|armature| {
            let (file, node) = armature.armature.split_once('#')?;
            let clips = armature
                .animations
                .as_deref()
                .and_then(|path| path.split_once('#'))
                .map_or(file, |(clips, _)| clips);
            Some(ArmatureSource {
                node: node.to_owned(),
                clips: server.load(clips.to_owned()),
                clip_names: clip_names.map(|names| names.0.clone()).unwrap_or_default(),
            })
        });
        commands.spawn((
            Name::new("ActorBody"),
            ActorBody {
                gltf: server.load(file.to_owned()),
                scene: scene.to_owned(),
                armature,
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
            commands
                .entity(entity)
                .insert(WorldAssetRoot(scene.clone()));
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
