use bevy::{
    animation::{AnimatedBy, AnimationTargetId, RepeatAnimation},
    gltf::Gltf,
    platform::collections::HashMap,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::time::Duration;

use super::ActorBody;
use crate::{actor_control::angle_between, ActorMotion};

/// Gaits by ground speed, in m/s (R18): walk up to `MAX_WALK_SPEED`, fast walk up to
/// `MAX_WALK_FAST_SPEED`, run above it. At exactly 3.5, the player's jog, a humanoid fast walks.
const MAX_WALK_SPEED: f32 = 2.0;
const MAX_WALK_FAST_SPEED: f32 = 3.5;

/// Slower than this along its facing, an actor stands.
const STAND_SPEED: f32 = 0.1;

/// Standing and turning faster than this, in radians per second, an actor plays its turn.
const TURN_SPEED: f32 = 1.0;

/// How long a landing holds its first frame, the deepest crouch, before it plays out the rise.
/// The crossfade from falling (0.1 s) blends away the start of the clip, so without a hold the
/// actor never reaches the bottom of the crouch.
const LANDING_HOLD: f32 = 0.1;

/// The rate the landing's rise plays at after the hold.
const LANDING_RISE_SPEED: f32 = 1.5;

/// What an actor's skeleton is playing (R33). A gait is a state of its own, so a plan reads which
/// one plays here.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[reflect(Component)]
pub enum ActorAnimationState {
    Stand,
    CombatStance,
    CombatStanceUnarmed,
    Turn,
    Walk,
    WalkFast,
    Run,
    Falling,
    Landing,
}

/// The state an actor plays when it has nothing else to do, chosen by its aspects (R38): combat
/// stance while in combat, for example. Without one, an actor stands.
#[derive(Component, Reflect, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[reflect(Component, Default)]
pub enum ActorIdle {
    #[default]
    Stand,
    CombatStance,
    CombatStanceUnarmed,
}

impl From<ActorIdle> for ActorAnimationState {
    fn from(idle: ActorIdle) -> Self {
        match idle {
            ActorIdle::Stand => Self::Stand,
            ActorIdle::CombatStance => Self::CombatStance,
            ActorIdle::CombatStanceUnarmed => Self::CombatStanceUnarmed,
        }
    }
}

/// How a state plays (R33, R34, R35): the first clip in `clips` the species has, faded in over
/// `fade`, looping or once. Fallback chains follow faery's `animation.ts`.
struct StateClips {
    clips: &'static [&'static str],
    fade: Duration,
    once: bool,
}

const fn looping(clips: &'static [&'static str], fade_ms: u64) -> StateClips {
    StateClips {
        clips,
        fade: Duration::from_millis(fade_ms),
        once: false,
    }
}

impl ActorAnimationState {
    const ALL: [Self; 9] = [
        Self::Stand,
        Self::CombatStance,
        Self::CombatStanceUnarmed,
        Self::Turn,
        Self::Walk,
        Self::WalkFast,
        Self::Run,
        Self::Falling,
        Self::Landing,
    ];

    fn clips(self) -> StateClips {
        match self {
            Self::Stand => looping(&["Stand"], 300),
            Self::CombatStance => looping(&["CombatStance", "Stand"], 300),
            Self::CombatStanceUnarmed => {
                looping(&["CombatStanceUnarmed", "CombatStance", "Stand"], 300)
            }
            Self::Turn => looping(&["Turn", "Stand"], 100),
            // Gaits fall back to each other, by speed, in `ActorClips::gait`.
            Self::Walk => looping(&["Walk"], 300),
            Self::WalkFast => looping(&["WalkFast"], 300),
            Self::Run => looping(&["Run"], 300),
            Self::Falling => looping(&["Fall", "Stand"], 500),
            // Optional: a species with no landing clip goes straight from falling to idle (R25).
            Self::Landing => StateClips {
                clips: &["Land"],
                fade: Duration::from_millis(100),
                once: true,
            },
        }
    }

    /// The ground speed a gait's clip was animated for, so its playback rate is the actor's speed
    /// over this, and feet do not slide (R18).
    fn nominal_speed(self) -> Option<f32> {
        match self {
            Self::Walk => Some(1.5),
            Self::WalkFast => Some(3.0),
            Self::Run => Some(5.0),
            _ => None,
        }
    }
}

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

/// The nodes of a skeleton's animation graph, one per clip the actor can play, and the clip each
/// state the species can play resolves to. Every clip hangs from the graph's root, and
/// `AnimationTransitions` crossfades by their weights.
#[derive(Debug, Component)]
pub(crate) struct ActorClips {
    states: HashMap<ActorAnimationState, StateClip>,
}

#[derive(Debug, Clone)]
struct StateClip {
    node: BlendNodeIndex,
    clip: Handle<AnimationClip>,
}

impl ActorClips {
    fn get(&self, state: ActorAnimationState) -> Option<&StateClip> {
        self.states.get(&state)
    }

    /// The gait for a ground speed, falling back to whichever gait clips the species has, as
    /// faery did; `None` to stand.
    fn gait(&self, speed: f32) -> Option<ActorAnimationState> {
        use ActorAnimationState::{Run, Walk, WalkFast};
        let speed = speed.abs();
        if speed < STAND_SPEED {
            return None;
        }
        let has = |state| self.states.contains_key(&state);
        if (speed > MAX_WALK_FAST_SPEED || (!has(Walk) && !has(WalkFast))) && has(Run) {
            return Some(Run);
        }
        if (speed > MAX_WALK_SPEED || !has(Walk)) && has(WalkFast) {
            return Some(WalkFast);
        }
        [Walk, WalkFast, Run].into_iter().find(|state| has(*state))
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
/// and starts it standing. States share a node where their chains reach the same clip.
pub(crate) fn build_actor_graph(
    mut commands: Commands,
    mut query: Query<(Entity, &ActorArmature, &mut AnimationPlayer), Without<ActorClips>>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<BlendGraph>>,
) {
    for (entity, armature, mut player) in query.iter_mut() {
        let Some(gltf) = gltfs.get(&armature.clips) else {
            continue;
        };
        // R41.
        let report = |missing: &str| {
            let mut clips: Vec<_> = gltf.named_animations.keys().collect();
            clips.sort();
            error!("Actor clip [{missing}] not found; clips: {clips:?}");
        };
        let mut renamed: Vec<_> = armature.clip_names.values().collect();
        renamed.sort();
        for name in renamed {
            if !gltf.named_animations.contains_key(name.as_str()) {
                report(name);
            }
        }
        let clip = |name: &str| {
            let name = armature.clip_names.get(name).map_or(name, String::as_str);
            Some((name.to_owned(), gltf.named_animations.get(name)?.clone()))
        };
        let mut graph = BlendGraph::new();
        let root = graph.root;
        let mut nodes = HashMap::<String, StateClip>::new();
        let mut states = HashMap::new();
        for state in ActorAnimationState::ALL {
            let Some((name, clip)) = state.clips().clips.iter().find_map(|name| clip(name)) else {
                continue;
            };
            let state_clip = nodes.entry(name).or_insert_with(|| StateClip {
                node: graph.add_clip(clip.clone(), 1.0, root),
                clip,
            });
            states.insert(state, state_clip.clone());
        }
        let Some(stand) = states
            .get(&ActorAnimationState::Stand)
            .map(|stand| stand.node)
        else {
            report("Stand");
            commands.entity(entity).remove::<ActorArmature>();
            continue;
        };
        let mut transitions = AnimationTransitions::new();
        transitions
            .play(&mut player, stand, Duration::ZERO)
            .repeat();
        commands.entity(entity).insert((
            ActorClips { states },
            ActorAnimationState::Stand,
            transitions,
            BlendGraphHandle(graphs.add(graph)),
        ));
    }
}

/// Plays each actor's state: falling while it falls (R25), then landing once on touchdown when
/// the species has a landing clip, then its gait, its turn or its idle. The gait follows its speed
/// along its facing over the last fixed tick, so backing up plays the clip backward; a new gait
/// crossfades in at the old one's phase of the stride (R19), and any other state starts its clip
/// from the beginning.
pub(crate) fn animate_actor_state(
    mut armatures: Query<(
        &ActorArmature,
        &ActorClips,
        &mut ActorAnimationState,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
    actors: Query<(&ActorMotion, Option<&ActorIdle>)>,
    clip_assets: Res<Assets<AnimationClip>>,
    time: Res<Time<Fixed>>,
) {
    use ActorAnimationState::{Falling, Landing, Turn};
    let dt = time.timestep().as_secs_f32();
    let duration = |clip: &StateClip| {
        clip_assets
            .get(&clip.clip)
            .map(AnimationClip::duration)
            .filter(|duration| *duration > 0.0)
    };
    for (armature, clips, mut state, mut player, mut transitions) in &mut armatures {
        let Ok((motion, idle)) = actors.get(armature.actor) else {
            continue;
        };
        let forward = Vec2::new(motion.facing.sin(), motion.facing.cos());
        let speed = (motion.position - motion.previous).xz().dot(forward) / dt;
        let turn = angle_between(motion.previous_facing, motion.facing) / dt;
        let gait = clips.gait(speed);
        // How long the landing has played, and whether it has finished.
        let landing = transitions
            .get_main_animation()
            .and_then(|node| player.animation(node))
            .map(|active| (active.elapsed(), active.is_finished()));
        let touched_down = *state == Falling && clips.get(Landing).is_some();
        // Moving cuts a landing short, once it has held its crouch.
        let still_landing = *state == Landing
            && landing.is_some_and(|(elapsed, finished)| {
                elapsed < LANDING_HOLD || (gait.is_none() && !finished)
            });
        let next = if motion.falling {
            Falling
        } else if touched_down || still_landing {
            Landing
        } else if let Some(gait) = gait {
            gait
        } else if turn.abs() > TURN_SPEED {
            Turn
        } else {
            idle.copied().unwrap_or_default().into()
        };
        // Every chain but the gaits' and landing's ends in `Stand`, which every species has.
        let Some(next_clip) = clips.get(next).or(clips.get(ActorAnimationState::Stand)) else {
            continue;
        };
        if next != *state {
            let old = clips
                .get(*state)
                .filter(|_| state.nominal_speed().is_some());
            *state = next;
            // The fraction of the stride the old gait had reached.
            let phase =
                old.and_then(|old| Some(player.animation(old.node)?.seek_time() / duration(old)?));
            if transitions.get_main_animation() != Some(next_clip.node) {
                let active = transitions.play(&mut player, next_clip.node, next.clips().fade);
                active.set_speed(1.0);
                if next.clips().once {
                    active.set_repeat(RepeatAnimation::Never);
                } else {
                    active.repeat();
                }
                if let Some((phase, new_duration)) = phase
                    .filter(|_| next.nominal_speed().is_some())
                    .zip(duration(next_clip))
                {
                    active.seek_to(phase * new_duration);
                }
            }
        }
        let Some(active) = player.animation_mut(next_clip.node) else {
            continue;
        };
        if let Some(nominal) = next.nominal_speed() {
            active.set_speed(speed / nominal);
        } else if next == Landing {
            // `elapsed` counts while the clip is paused.
            active.set_speed(if active.elapsed() < LANDING_HOLD {
                0.0
            } else {
                LANDING_RISE_SPEED
            });
        }
    }
}
