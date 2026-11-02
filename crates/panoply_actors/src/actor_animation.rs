use bevy::{
    animation::{AnimatedBy, AnimationTargetId},
    gltf::Gltf,
    platform::collections::HashMap,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::time::Duration;

use super::ActorBody;
use crate::ActorMotion;

/// TODO(PAN3j): the clip follows the actor's state. Until then, an actor stands unless it moves.
const STAND_CLIP: &str = "Stand";

/// Gaits by ground speed, in m/s (R18): walk up to `MAX_WALK_SPEED`, fast walk up to
/// `MAX_WALK_FAST_SPEED`, run above it. At exactly 3.5, the player's jog, a humanoid fast walks.
const MAX_WALK_SPEED: f32 = 2.0;
const MAX_WALK_FAST_SPEED: f32 = 3.5;

/// The ground speed each gait's clip was animated for, so its playback rate is the actor's speed
/// over this, and feet do not slide (R18).
const NOMINAL_WALK_SPEED: f32 = 1.5;
const NOMINAL_WALK_FAST_SPEED: f32 = 3.0;
const NOMINAL_RUN_SPEED: f32 = 5.0;

/// Slower than this along its facing, an actor stands.
const STAND_SPEED: f32 = 0.1;

/// How long one clip takes to fade into the next (R34).
const CROSSFADE: Duration = Duration::from_millis(300);

/// Where a body's skeleton is and where its clips come from, from the `Armature` and `ClipNames`
/// aspects.
#[derive(Debug)]
pub struct ArmatureSource {
    /// Name of the skeleton's root node in the body scene.
    pub node: String,
    /// The glTF holding the species' clips, often a file shared by several species.
    pub clips: Handle<Gltf>,
    /// The species' names for clips, where they differ from the engine's.
    pub clip_names: HashMap<String, String>,
}

/// The root of an actor's skeleton, which holds its `AnimationPlayer`.
#[derive(Debug, Component)]
pub struct ActorArmature {
    /// The actor whose movement the skeleton follows.
    actor: Entity,
    clips: Handle<Gltf>,
    clip_names: HashMap<String, String>,
}

/// The nodes of a skeleton's animation graph, one per clip the actor can play. Every clip hangs
/// from the graph's root, and `AnimationTransitions` crossfades by their weights.
#[derive(Debug, Component)]
pub(crate) struct ActorClips {
    stand: AnimationNodeIndex,
    walk: Option<Gait>,
    walk_fast: Option<Gait>,
    run: Option<Gait>,
}

/// A locomotion clip and the ground speed it was animated for.
#[derive(Debug, Clone)]
struct Gait {
    node: AnimationNodeIndex,
    clip: Handle<AnimationClip>,
    nominal_speed: f32,
}

impl ActorClips {
    /// The gait for a ground speed, falling back to whichever gait clips the species has, as
    /// faery did; `None` to stand.
    fn gait(&self, speed: f32) -> Option<&Gait> {
        let speed = speed.abs();
        if speed < STAND_SPEED {
            return None;
        }
        let (walk, walk_fast, run) = (
            self.walk.as_ref(),
            self.walk_fast.as_ref(),
            self.run.as_ref(),
        );
        if (speed > MAX_WALK_FAST_SPEED || (walk.is_none() && walk_fast.is_none())) && run.is_some()
        {
            return run;
        }
        if (speed > MAX_WALK_SPEED || walk.is_none()) && walk_fast.is_some() {
            return walk_fast;
        }
        walk.or(walk_fast).or(run)
    }

    fn gait_of(&self, node: AnimationNodeIndex) -> Option<&Gait> {
        [&self.walk, &self.walk_fast, &self.run]
            .into_iter()
            .flatten()
            .find(|gait| gait.node == node)
    }
}

/// Binds a body's skeleton to an `AnimationPlayer` once its scene has spawned. A body file with
/// no clips of its own gets no animation targets from Bevy's loader, so they are added here:
/// each bone's `AnimationTargetId` is its path of names from the skeleton's root, as the loader
/// computes it for the clips' file, so clips bind to any skeleton with the same bone names (D28).
pub fn bind_actor_skeleton(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    bodies: Query<(&ActorBody, &ChildOf)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok((body, actor)) = bodies.get(ready.entity) else {
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
            actor: actor.parent(),
            clips: armature.clips.clone(),
            clip_names: armature.clip_names.clone(),
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

/// Builds each skeleton's animation graph from the species' clips, once their file has loaded,
/// and starts it standing.
pub(crate) fn build_actor_graph(
    mut commands: Commands,
    mut query: Query<(Entity, &ActorArmature, &mut AnimationPlayer), Without<ActorClips>>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (entity, armature, mut player) in query.iter_mut() {
        let Some(gltf) = gltfs.get(&armature.clips) else {
            continue;
        };
        let clip = |name: &str| {
            let name = armature.clip_names.get(name).map_or(name, String::as_str);
            gltf.named_animations.get(name).cloned()
        };
        let Some(stand) = clip(STAND_CLIP) else {
            let mut clips: Vec<_> = gltf.named_animations.keys().collect();
            clips.sort();
            error!("Actor clip [{STAND_CLIP}] not found; clips: {clips:?}");
            commands.entity(entity).remove::<ActorArmature>();
            continue;
        };
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        let stand = graph.add_clip(stand, 1.0, root);
        let mut gait = |name: &str, nominal_speed: f32| {
            let clip = clip(name)?;
            Some(Gait {
                node: graph.add_clip(clip.clone(), 1.0, root),
                clip,
                nominal_speed,
            })
        };
        let clips = ActorClips {
            walk: gait("Walk", NOMINAL_WALK_SPEED),
            walk_fast: gait("WalkFast", NOMINAL_WALK_FAST_SPEED),
            run: gait("Run", NOMINAL_RUN_SPEED),
            stand,
        };
        let mut transitions = AnimationTransitions::new();
        transitions
            .play(&mut player, stand, Duration::ZERO)
            .repeat();
        commands.entity(entity).insert((
            clips,
            transitions,
            AnimationGraphHandle(graphs.add(graph)),
        ));
    }
}

/// Plays the clip for each actor's gait, from its speed along its facing over the last fixed
/// tick, so backing up plays the clip backward. A new gait crossfades in at the old one's phase
/// of the stride (R19); standing starts its clip from the beginning.
pub(crate) fn animate_actor_gait(
    mut armatures: Query<(
        &ActorArmature,
        &ActorClips,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
    actors: Query<&ActorMotion>,
    clip_assets: Res<Assets<AnimationClip>>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.timestep().as_secs_f32();
    let duration = |gait: &Gait| {
        clip_assets
            .get(&gait.clip)
            .map(AnimationClip::duration)
            .filter(|duration| *duration > 0.0)
    };
    for (armature, clips, mut player, mut transitions) in &mut armatures {
        let Ok(motion) = actors.get(armature.actor) else {
            continue;
        };
        let forward = Vec2::new(motion.facing.sin(), motion.facing.cos());
        let speed = (motion.position - motion.previous).xz().dot(forward) / dt;
        let gait = clips.gait(speed);
        let node = gait.map_or(clips.stand, |gait| gait.node);
        let current = transitions.get_main_animation();
        if current != Some(node) {
            // The fraction of the stride the old gait had reached.
            let phase = current
                .and_then(|current| clips.gait_of(current))
                .and_then(|old| Some(player.animation(old.node)?.seek_time() / duration(old)?));
            let active = transitions.play(&mut player, node, CROSSFADE).repeat();
            if let Some((phase, new)) = phase.zip(gait) {
                if let Some(new_duration) = duration(new) {
                    active.seek_to(phase * new_duration);
                }
            }
        }
        if let (Some(gait), Some(active)) = (gait, player.animation_mut(node)) {
            active.set_speed(speed / gait.nominal_speed);
        }
    }
}
