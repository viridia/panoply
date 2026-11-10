use bevy::{
    audio::{SpatialScale, Volume},
    gltf::Gltf,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use panoply_core::Viewpoint;
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};

use super::{
    container_aspect::ContainerState, scenery_element::SceneryElementMesh, FIXTURE_TYPE, WALL_TYPE,
};

/// The state of an instance that drives an animation.
#[derive(Debug, Reflect, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimationInput {
    /// A container's [`ContainerState::open`].
    #[default]
    Open,
}

impl AnimationInput {
    fn read(self, container: Option<&ContainerState>) -> bool {
        match self {
            AnimationInput::Open => container.is_some_and(|state| state.open),
        }
    }
}

/// What happens when an animation's input changes to `to`.
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct AnimationTransition {
    /// The value being changed to.
    pub to: bool,

    /// How long to wait before playing, in seconds.
    #[serde(default)]
    pub delay: f32,

    /// Heard when the clips start.
    #[serde(default)]
    pub sound: Option<TransitionSound>,
}

/// A sound that accompanies an [`AnimationTransition`].
#[derive(Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct TransitionSound {
    /// The audio file.
    pub asset: String,

    /// How long after the clips start to play it, in seconds.
    #[serde(default)]
    pub delay: f32,

    /// Linear gain at close range.
    #[serde(default = "full_volume")]
    pub volume: f32,
}

impl Default for TransitionSound {
    fn default() -> Self {
        Self {
            asset: String::new(),
            delay: 0.,
            volume: full_volume(),
        }
    }
}

fn full_volume() -> f32 {
    1.
}

/// Clips of a model that play together, as its exemplar describes them.
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct SceneryAnimation {
    /// Names of the clips, in the file of the model.
    pub clips: Vec<String>,

    /// The state that plays the clips forward when true and backward when false.
    #[serde(default)]
    pub input: Option<AnimationInput>,

    /// Plays the clips repeatedly from spawn, instead of following an input.
    #[serde(default, rename = "loop")]
    pub looping: bool,

    #[serde(default)]
    pub transitions: Vec<AnimationTransition>,
}

impl SceneryAnimation {
    fn bind_transition(&self, to: bool, asset_server: &AssetServer) -> BoundTransition {
        let transition = self
            .transitions
            .iter()
            .find(|transition| transition.to == to);
        BoundTransition {
            delay: transition.map_or(0., |transition| transition.delay),
            sound: transition
                .and_then(|transition| transition.sound.as_ref())
                .map(|sound| BoundSound {
                    source: asset_server.load(&sound.asset),
                    delay: sound.delay,
                    volume: sound.volume,
                }),
        }
    }
}

/// How the parts of a fixture or wall tile's model move.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct SceneryAnimations(pub Vec<SceneryAnimation>);

impl Aspect for SceneryAnimations {
    fn name(&self) -> &str {
        "SceneryAnimations"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == FIXTURE_TYPE || meta_type == WALL_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<SceneryAnimations> =
            RemoveComponent::<SceneryAnimations>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// A [`TransitionSound`], its file loading from the time the model binds so that it is ready to
/// play.
#[derive(Clone)]
struct BoundSound {
    source: Handle<AudioSource>,
    delay: f32,
    volume: f32,
}

/// An [`AnimationTransition`], bound to its model.
struct BoundTransition {
    delay: f32,
    sound: Option<BoundSound>,
}

/// One [`SceneryAnimation`] that follows an input, bound to its model.
struct InputAnimation {
    nodes: Vec<BlendNodeIndex>,
    input: AnimationInput,
    on_true: BoundTransition,
    on_false: BoundTransition,
    /// The input's value, as last seen.
    value: bool,
    /// Seconds left before the clips play for a change of the input.
    wait: Option<f32>,
}

/// An instance's animations that follow inputs, and the player on its model.
#[derive(Component)]
pub(crate) struct BoundAnimations {
    player: Entity,
    animations: Vec<InputAnimation>,
}

/// Once a model has spawned, gives its `AnimationPlayer`, which Bevy's loader puts on the model's
/// root, a graph of its instance's clips. Looping clips start now; those whose input is already
/// true start at their ends.
pub(crate) fn bind_scenery_animations(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    meshes: Query<(&SceneryElementMesh, &ChildOf)>,
    instances: Query<(&SceneryAnimations, Option<&ContainerState>)>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<BlendGraph>>,
    asset_server: Res<AssetServer>,
) {
    let Ok((mesh, element)) = meshes.get(ready.entity) else {
        return;
    };
    let Ok((animations, container)) = instances.get(element.parent()) else {
        return;
    };
    let Some(gltf) = gltfs.get(&mesh.handle) else {
        return;
    };
    let Some(player_entity) = children
        .iter_descendants(ready.entity)
        .find(|entity| players.contains(*entity))
    else {
        error!("Animated scenery {:?} has no player", animations.0);
        return;
    };
    let Ok(mut player) = players.get_mut(player_entity) else {
        return;
    };

    let mut graph = BlendGraph::new();
    let mut bound = Vec::new();
    for animation in &animations.0 {
        let value = animation
            .input
            .is_some_and(|input| input.read(container));
        let mut nodes = Vec::with_capacity(animation.clips.len());
        for name in &animation.clips {
            let Some(handle) = gltf.named_animations.get(name.as_str()) else {
                error!("Animation clip [{name}] not found");
                continue;
            };
            let node = graph.add_clip(handle.clone(), 1., graph.root);
            nodes.push(node);
            if animation.looping {
                player.play(node).repeat();
            } else if let (true, Some(clip)) = (value, clips.get(handle)) {
                player.play(node).seek_to(clip.duration());
            }
        }
        if let Some(input) = animation.input {
            bound.push(InputAnimation {
                nodes,
                input,
                on_true: animation.bind_transition(true, &asset_server),
                on_false: animation.bind_transition(false, &asset_server),
                value,
                wait: None,
            });
        }
    }
    commands
        .entity(player_entity)
        .insert(BlendGraphHandle(graphs.add(graph)));
    commands.entity(element.parent()).insert(BoundAnimations {
        player: player_entity,
        animations: bound,
    });
}

/// Plays clips whose input has changed, after the transition's delay, from wherever they have got
/// to: forward for true, backward for false. A clip that has run its course holds its last pose.
/// Queues the transition's sound as the clips start.
pub(crate) fn drive_scenery_animations(
    time: Res<Time>,
    mut commands: Commands,
    mut instances: Query<(
        &mut BoundAnimations,
        Option<&ContainerState>,
        &GlobalTransform,
    )>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for (mut bound, container, transform) in &mut instances {
        let bound = &mut *bound;
        for animation in &mut bound.animations {
            let value = animation.input.read(container);
            let transition = if value {
                &animation.on_true
            } else {
                &animation.on_false
            };
            if value != animation.value {
                animation.value = value;
                animation.wait = Some(transition.delay);
            }
            let Some(wait) = &mut animation.wait else {
                continue;
            };
            *wait -= time.delta_secs();
            if *wait > 0. {
                continue;
            }
            animation.wait = None;
            if let Some(sound) = &transition.sound {
                commands.spawn((
                    Name::new("TransitionSound"),
                    Transform::from_translation(transform.translation()),
                    PendingSound {
                        sound: sound.clone(),
                        wait: sound.delay,
                    },
                ));
            }
            let Ok(mut player) = players.get_mut(bound.player) else {
                continue;
            };
            for &node in &animation.nodes {
                let active = player.play(node);
                let time = active.seek_time().max(0.);
                // A finished animation no longer advances; `replay` makes it run again.
                active.replay();
                active
                    .seek_to(time)
                    .set_speed(if value { 1. } else { -1. });
            }
        }
    }
}

/// A sound whose [`TransitionSound::delay`] is still running. An entity of its own, so that a
/// sound already on its way is heard even if the input changes back.
#[derive(Component)]
pub(crate) struct PendingSound {
    sound: BoundSound,
    wait: f32,
}

/// Within this distance of the viewpoint, in metres, a sound is at full volume.
const SOUND_NEAR: f32 = 2.;

/// At this distance from the viewpoint, in metres, and beyond, a sound is silent.
const SOUND_FAR: f32 = 16.;

/// The exponent of the curve between [`SOUND_NEAR`] and [`SOUND_FAR`].
const SOUND_FALLOFF: f32 = 1.9;

/// Shrinks positions so that every ear is within rodio's 1 unit of the emitter, where its
/// inverse-square falloff has no effect: [`sound_gain`] sets the loudness and the spatial sink only
/// pans, which depends on the ratio of the distances and so is unaffected by the scale. Rodio's
/// falloff measured from the camera would leave a chest 11 m away almost silent.
const SOUND_PAN_ONLY: SpatialScale = SpatialScale::new(0.001);

/// The gain of a sound of `volume`, `distance` metres from the viewpoint. Measured from the
/// viewpoint, which follows the player, so that zooming the camera does not change it.
fn sound_gain(volume: f32, distance: f32) -> f32 {
    let t = ((distance - SOUND_NEAR) / (SOUND_FAR - SOUND_NEAR)).clamp(0., 1.);
    volume * (1. - t).powf(SOUND_FALLOFF)
}

/// Starts each [`PendingSound`] whose delay is over, at the loudness its distance gives it then.
pub(crate) fn start_pending_sounds(
    time: Res<Time>,
    viewpoint: Res<Viewpoint>,
    mut commands: Commands,
    mut pending: Query<(Entity, &mut PendingSound, &Transform)>,
) {
    for (entity, mut sound, transform) in &mut pending {
        sound.wait -= time.delta_secs();
        if sound.wait > 0. {
            continue;
        }
        let distance = viewpoint.position.distance(transform.translation);
        let gain = sound_gain(sound.sound.volume, distance);
        if gain <= 0. {
            commands.entity(entity).despawn();
            continue;
        }
        commands
            .entity(entity)
            .remove::<PendingSound>()
            .insert((
                AudioPlayer::new(sound.sound.source.clone()),
                PlaybackSettings {
                    volume: Volume::Linear(gain),
                    spatial: true,
                    spatial_scale: Some(SOUND_PAN_ONLY),
                    ..PlaybackSettings::DESPAWN
                },
            ));
    }
}
