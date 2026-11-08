use bevy::{gltf::Gltf, prelude::*, world_serialization::WorldInstanceReady};
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};

use super::{scenery_element::SceneryElementMesh, FIXTURE_TYPE};

/// A fixture that opens and closes, such as a chest.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct Container {
    /// Name of the clip that opens it, in the file of its model.
    pub clip: String,
}

impl Aspect for Container {
    fn name(&self) -> &str {
        "Container"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Container> = RemoveComponent::<Container>::new();
        entity.insert(self.clone());
        entity.insert_if_new(ContainerState::default());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Whether a container is open, kept apart from [`Container`] so that reapplying aspects leaves it
/// as it was.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Component, Default)]
pub struct ContainerState {
    /// True when open or opening.
    pub open: bool,
}

/// The animation that moves a container's lid.
#[derive(Component)]
pub(crate) struct ContainerClip {
    player: Entity,
    node: AnimationNodeIndex,
}

/// Opens a closed container, or closes an open one.
#[derive(EntityEvent)]
pub struct ContainerToggle {
    /// The container.
    pub entity: Entity,
}

/// Once a container's model has spawned, gives its `AnimationPlayer`, which Bevy's loader puts on
/// the model's root, a graph of the opening clip. A container already open starts at the clip's
/// end.
pub(crate) fn bind_container_clip(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    meshes: Query<(&SceneryElementMesh, &ChildOf)>,
    containers: Query<(&Container, &ContainerState)>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let Ok((mesh, element)) = meshes.get(ready.entity) else {
        return;
    };
    let Ok((container, state)) = containers.get(element.parent()) else {
        return;
    };
    let Some(clip) = gltfs
        .get(&mesh.handle)
        .and_then(|gltf| gltf.named_animations.get(container.clip.as_str()))
    else {
        error!("Container clip [{}] not found", container.clip);
        return;
    };
    let Some(player) = children
        .iter_descendants(ready.entity)
        .find(|entity| players.contains(*entity))
    else {
        error!("Container clip [{}] has no player", container.clip);
        return;
    };
    let (graph, node) = AnimationGraph::from_clip(clip.clone());
    commands
        .entity(player)
        .insert(AnimationGraphHandle(graphs.add(graph)));
    commands
        .entity(element.parent())
        .insert(ContainerClip { player, node });
    if state.open {
        if let (Ok(mut player), Some(clip)) = (players.get_mut(player), clips.get(clip)) {
            player.play(node).seek_to(clip.duration());
        }
    }
}

/// Flips a container and plays its clip from wherever it has got to: forward to open, backward to
/// close. A clip that has run its course holds its last pose.
pub(crate) fn toggle_container(
    toggle: On<ContainerToggle>,
    mut containers: Query<(&mut ContainerState, Option<&ContainerClip>)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok((mut state, clip)) = containers.get_mut(toggle.entity) else {
        return;
    };
    state.open = !state.open;
    let Some(clip) = clip else {
        return;
    };
    let Ok(mut player) = players.get_mut(clip.player) else {
        return;
    };
    let active = player.play(clip.node);
    let time = active.seek_time().max(0.);
    // A finished animation no longer advances; `replay` makes it run again.
    active.replay();
    active
        .seek_to(time)
        .set_speed(if state.open { 1. } else { -1. });
}
