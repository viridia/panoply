use bevy::{
    annotations::DisplayName,
    audio::{SpatialScale, Volume},
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    gltf::Gltf,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use panoply_core::Viewpoint;
use panoply_exemplar::{annotations::AspectIcon, *};
use panoply_saga::{CellId, Formula, Reactions};
use serde::{Deserialize, Serialize};

use super::scenery_element::SceneryElementMesh;

/// What happens when an animation's `when` changes to `to`.
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct AnimationTransition {
    /// The value being changed to.
    #[reflect(@DisplayName("To"))]
    pub to: bool,

    /// How long to wait before playing, in seconds.
    #[serde(default)]
    #[reflect(@DisplayName("Delay"))]
    pub delay: f32,

    /// Heard when the clips start.
    #[serde(default)]
    #[reflect(@DisplayName("Sound"))]
    pub sound: Option<TransitionSound>,
}

/// A sound that accompanies an [`AnimationTransition`].
#[derive(Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct TransitionSound {
    /// The audio file.
    #[reflect(@DisplayName("Audio file"))]
    pub asset: String,

    /// How long after the clips start to play it, in seconds.
    #[serde(default)]
    #[reflect(@DisplayName("Delay"))]
    pub delay: f32,

    /// Linear gain at close range.
    #[serde(default = "full_volume")]
    #[reflect(@DisplayName("Volume"))]
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

/// Clips of a model that play together, as its exemplar describes them. Not deserialized by
/// serde, so that loaders reach its formula through reflection; a field a file leaves out takes
/// its default.
#[derive(Debug, Reflect, Clone, Default, Serialize)]
#[reflect(Default, Serialize)]
pub struct SceneryAnimation {
    /// Names of the clips, in the file of the model.
    #[reflect(@DisplayName("Clips"))]
    pub clips: Vec<String>,

    /// A formula over the instance that plays the clips forward while true and backward while
    /// false (D116).
    #[reflect(@DisplayName("When"))]
    pub when: Option<Formula<bool>>,

    /// Plays the clips repeatedly from spawn, instead of following a `when`.
    #[reflect(@DisplayName("Loop"))]
    pub looping: bool,

    #[reflect(@DisplayName("Transitions"))]
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
#[derive(Component, Debug, Reflect, Clone, Default, Serialize)]
#[reflect(Aspect, Default, Serialize)]
#[reflect(@AspectIcon("editor/icons/lucide/clapperboard.png"))]
pub struct SceneryAnimations(pub Vec<SceneryAnimation>);

impl NativeAspect for SceneryAnimations {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Fixture || kind == InstanceKind::Wall
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

/// One [`SceneryAnimation`] that follows its `when`, bound to its model.
struct WhenAnimation {
    /// Each clip's node, with the clip's length.
    nodes: Vec<(BlendNodeIndex, f32)>,
    /// The `when`, kept up to date by the reaction pass.
    cell: CellId,
    on_true: BoundTransition,
    on_false: BoundTransition,
    /// The `when`'s value, as last seen; `None` before its first.
    value: Option<bool>,
    /// Seconds left before the clips play for a change of the `when`.
    wait: Option<f32>,
}

/// An instance's animations that follow a `when`, and the player on its model. Their cells go
/// with it.
#[derive(Component)]
#[component(on_discard = remove_animation_cells)]
pub(crate) struct BoundAnimations {
    player: Entity,
    animations: Vec<WhenAnimation>,
}

fn remove_animation_cells(mut world: DeferredWorld, context: HookContext) {
    let Some(bound) = world.get::<BoundAnimations>(context.entity) else {
        return;
    };
    let cells: Vec<CellId> = bound
        .animations
        .iter()
        .map(|animation| animation.cell)
        .collect();
    if let Some(mut reactions) = world.get_resource_mut::<Reactions>() {
        for cell in cells {
            reactions.remove(cell);
        }
    }
}

/// Once a model has spawned, gives its `AnimationPlayer`, which Bevy's loader puts on the model's
/// root, a graph of its instance's clips, and each `when` a cell. Looping clips start now. A model
/// without a player is one the clips do not move, such as the wall behind a switch's lever, and is
/// skipped.
pub(crate) fn bind_scenery_animations(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    meshes: Query<(&SceneryElementMesh, &ChildOf)>,
    instances: Query<&SceneryAnimations>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<BlendGraph>>,
    mut reactions: ResMut<Reactions>,
    asset_server: Res<AssetServer>,
) {
    let Ok((mesh, element)) = meshes.get(ready.entity) else {
        return;
    };
    let instance = element.parent();
    let Ok(animations) = instances.get(instance) else {
        return;
    };
    let Some(gltf) = gltfs.get(&mesh.handle) else {
        return;
    };
    let Some(player_entity) = children
        .iter_descendants(ready.entity)
        .find(|entity| players.contains(*entity))
    else {
        return;
    };
    let Ok(mut player) = players.get_mut(player_entity) else {
        return;
    };

    let mut graph = BlendGraph::new();
    let mut bound = Vec::new();
    for animation in &animations.0 {
        let mut nodes = Vec::with_capacity(animation.clips.len());
        for name in &animation.clips {
            let Some(handle) = gltf.named_animations.get(name.as_str()) else {
                error!("Animation clip [{name}] not found");
                continue;
            };
            let node = graph.add_clip(handle.clone(), 1., graph.root);
            let length = clips.get(handle).map_or(0., AnimationClip::duration);
            nodes.push((node, length));
            if animation.looping {
                player.play(node).repeat();
            }
        }
        if let Some(when) = &animation.when {
            bound.push(WhenAnimation {
                nodes,
                cell: reactions.add(instance, when.clone()),
                on_true: animation.bind_transition(true, &asset_server),
                on_false: animation.bind_transition(false, &asset_server),
                value: None,
                wait: None,
            });
        }
    }
    commands
        .entity(player_entity)
        .insert(BlendGraphHandle(graphs.add(graph)));
    commands.entity(instance).insert(BoundAnimations {
        player: player_entity,
        animations: bound,
    });
}

/// Plays clips whose `when` has changed, after the transition's delay, from wherever they have
/// got to: forward for true, backward for false. A clip that has run its course holds its last
/// pose. Queues the transition's sound as the clips start. A `when` first seen true snaps its
/// clips to their ends, so that an instance spawns, or a save loads, in the pose it gives.
pub(crate) fn drive_scenery_animations(
    time: Res<Time>,
    mut commands: Commands,
    reactions: Res<Reactions>,
    mut instances: Query<(&mut BoundAnimations, &GlobalTransform)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for (mut bound, transform) in &mut instances {
        let bound = &mut *bound;
        for animation in &mut bound.animations {
            let Some(value) = reactions.value(animation.cell) else {
                continue;
            };
            let Some(last) = animation.value.replace(value) else {
                if let (true, Ok(mut player)) = (value, players.get_mut(bound.player)) {
                    for &(node, length) in &animation.nodes {
                        player.play(node).seek_to(length);
                    }
                }
                continue;
            };
            let transition = if value {
                &animation.on_true
            } else {
                &animation.on_false
            };
            if value != last {
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
            for &(node, length) in &animation.nodes {
                let active = player.play(node);
                // A finished clip stops a little past its end: past the start when it ran back,
                // past the length when it ran forward. Bevy wraps a reversed clip still past its
                // length to near zero, which would end the reverse at once.
                let time = active.seek_time().clamp(0., length);
                // A finished animation no longer advances; `replay` makes it run again.
                active.replay();
                active.seek_to(time).set_speed(if value { 1. } else { -1. });
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
        commands.entity(entity).remove::<PendingSound>().insert((
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

#[cfg(test)]
mod tests {
    use bevy::reflect::{serde::TypedReflectDeserializer, GetTypeRegistration, TypeRegistry};
    use panoply_saga::{FormulaQueue, HostState};
    use serde::de::DeserializeSeed;

    use super::*;

    /// Animations read as a catalog's are: through reflection, their formulas compiled.
    fn load(json: &str) -> SceneryAnimations {
        let mut registry = TypeRegistry::new();
        registry.register::<SceneryAnimations>();
        let registration = registry
            .get(SceneryAnimations::get_type_registration().type_id())
            .unwrap();
        let mut formulas = FormulaQueue::default();
        let value = {
            let reader = formulas.track_json(json.as_bytes());
            let mut deserializer = serde_json::Deserializer::from_reader(reader);
            TypedReflectDeserializer::with_processor(registration, &registry, &mut formulas)
                .deserialize(&mut deserializer)
                .unwrap()
        };
        futures_lite::future::block_on(formulas.compile(
            "test.json",
            &HostState::new(),
            json.as_bytes(),
            &[],
        ));
        SceneryAnimations::from_reflect(&*value).unwrap()
    }

    #[test]
    fn a_when_compiles_and_fields_left_out_take_their_defaults() {
        let animations = load(
            r#"[
  {"clips": ["Open"], "when": "1 < 2", "transitions": [{"to": true, "sound": {"asset": "a.ogg"}}]},
  {"clips": ["Blades"], "looping": true}
]"#,
        );
        let [opening, turning] = &animations.0[..] else {
            panic!("{animations:?}");
        };
        assert!(opening.when.as_ref().unwrap().module().is_some());
        assert!(!opening.looping);
        let sound = opening.transitions[0].sound.as_ref().unwrap();
        assert_eq!((opening.transitions[0].delay, sound.volume), (0., 1.));
        assert!(turning.when.is_none() && turning.looping);
        assert!(turning.transitions.is_empty());
    }
}
