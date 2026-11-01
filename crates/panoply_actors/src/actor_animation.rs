use bevy::{
    animation::{AnimatedBy, AnimationTargetId},
    gltf::Gltf,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use super::ActorBody;

/// TODO(PAN3j): the clip follows the actor's state. Until then, every actor stands.
const IDLE_CLIP: &str = "Stand";

/// Where a body's skeleton is and where its clips come from, from the `Armature` aspect.
#[derive(Debug)]
pub struct ArmatureSource {
    /// Name of the skeleton's root node in the body scene.
    pub node: String,
    /// The glTF holding the species' clips, often a file shared by several species.
    pub clips: Handle<Gltf>,
}

/// The root of an actor's skeleton, which holds its `AnimationPlayer`.
#[derive(Debug, Component)]
pub struct ActorArmature {
    clips: Handle<Gltf>,
}

/// Binds a body's skeleton to an `AnimationPlayer` once its scene has spawned. A body file with
/// no clips of its own gets no animation targets from Bevy's loader, so they are added here:
/// each bone's `AnimationTargetId` is its path of names from the skeleton's root, as the loader
/// computes it for the clips' file, so clips bind to any skeleton with the same bone names (D28).
pub fn bind_actor_skeleton(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    bodies: Query<&ActorBody>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(body) = bodies.get(ready.entity) else {
        return;
    };
    let Some(armature) = &body.armature else {
        return;
    };
    let Some(root) = children.iter_descendants(ready.entity).find(|entity| {
        names
            .get(*entity)
            .is_ok_and(|name| name.as_str() == armature.node)
    }) else {
        error!("Actor skeleton [{}] not found in its body", armature.node);
        return;
    };
    let mut path = vec![names.get(root).unwrap().clone()];
    bind_targets(&mut commands, root, root, &mut path, &children, &names);
    commands.entity(root).insert((
        AnimationPlayer::default(),
        ActorArmature {
            clips: armature.clips.clone(),
        },
    ));
}

fn bind_targets(
    commands: &mut Commands,
    player: Entity,
    entity: Entity,
    path: &mut Vec<Name>,
    children: &Query<&Children>,
    names: &Query<&Name>,
) {
    commands.entity(entity).insert((
        AnimationTargetId::from_names(path.iter()),
        AnimatedBy(player),
    ));
    for child in children.get(entity).into_iter().flatten() {
        if let Ok(name) = names.get(*child) {
            path.push(name.clone());
            bind_targets(commands, player, *child, path, children, names);
            path.pop();
        }
    }
}

/// Starts the idle clip on each skeleton, once its clips' file has loaded.
pub fn play_actor_idle(
    mut commands: Commands,
    mut query: Query<(Entity, &ActorArmature, &mut AnimationPlayer), Without<AnimationGraphHandle>>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (entity, armature, mut player) in query.iter_mut() {
        let Some(gltf) = gltfs.get(&armature.clips) else {
            continue;
        };
        let Some(clip) = gltf.named_animations.get(IDLE_CLIP) else {
            let mut clips: Vec<_> = gltf.named_animations.keys().collect();
            clips.sort();
            error!("Actor clip [{IDLE_CLIP}] not found; clips: {clips:?}");
            commands.entity(entity).remove::<ActorArmature>();
            continue;
        };
        let (graph, node) = AnimationGraph::from_clip(clip.clone());
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(graphs.add(graph)));
        player.play(node).repeat();
    }
}
