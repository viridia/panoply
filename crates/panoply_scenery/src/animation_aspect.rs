use bevy::{gltf::Gltf, prelude::*, world_serialization::WorldInstanceReady};
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
    fn delay(&self, to: bool) -> f32 {
        self.transitions
            .iter()
            .find(|transition| transition.to == to)
            .map_or(0., |transition| transition.delay)
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

/// One [`SceneryAnimation`] that follows an input, bound to its model.
struct InputAnimation {
    nodes: Vec<AnimationNodeIndex>,
    input: AnimationInput,
    delay_true: f32,
    delay_false: f32,
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
    mut graphs: ResMut<Assets<AnimationGraph>>,
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

    let mut graph = AnimationGraph::new();
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
                delay_true: animation.delay(true),
                delay_false: animation.delay(false),
                value,
                wait: None,
            });
        }
    }
    commands
        .entity(player_entity)
        .insert(AnimationGraphHandle(graphs.add(graph)));
    commands.entity(element.parent()).insert(BoundAnimations {
        player: player_entity,
        animations: bound,
    });
}

/// Plays clips whose input has changed, after the transition's delay, from wherever they have got
/// to: forward for true, backward for false. A clip that has run its course holds its last pose.
pub(crate) fn drive_scenery_animations(
    time: Res<Time>,
    mut instances: Query<(&mut BoundAnimations, Option<&ContainerState>)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for (mut bound, container) in &mut instances {
        let bound = &mut *bound;
        for animation in &mut bound.animations {
            let value = animation.input.read(container);
            if value != animation.value {
                animation.value = value;
                animation.wait = Some(if value {
                    animation.delay_true
                } else {
                    animation.delay_false
                });
            }
            let Some(wait) = &mut animation.wait else {
                continue;
            };
            *wait -= time.delta_secs();
            if *wait > 0. {
                continue;
            }
            animation.wait = None;
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
