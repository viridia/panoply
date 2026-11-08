use bevy::{gltf::Gltf, prelude::*, world_serialization::WorldInstanceReady};
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};

use super::{scenery_element::SceneryElementMesh, FIXTURE_TYPE, WALL_TYPE};

/// A fixture or wall tile that opens and closes, such as a chest or a sarcophagus.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct Container {
    /// Names of the clips that open it, played together, in the file of its model.
    pub clips: Vec<String>,
}

impl Aspect for Container {
    fn name(&self) -> &str {
        "Container"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == FIXTURE_TYPE || meta_type == WALL_TYPE
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

/// The animations that move a container's lid or doors.
#[derive(Component)]
pub(crate) struct ContainerClips {
    player: Entity,
    nodes: Vec<AnimationNodeIndex>,
}

/// Opens a closed container, or closes an open one.
#[derive(EntityEvent)]
pub struct ContainerToggle {
    /// The container.
    pub entity: Entity,
}

/// Once a container's model has spawned, gives its `AnimationPlayer`, which Bevy's loader puts on
/// the model's root, a graph of the opening clips. A container already open starts at their ends.
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
    let Some(gltf) = gltfs.get(&mesh.handle) else {
        return;
    };
    let Some(handles) = container
        .clips
        .iter()
        .map(|name| {
            let clip = gltf.named_animations.get(name.as_str());
            if clip.is_none() {
                error!("Container clip [{name}] not found");
            }
            clip.cloned()
        })
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let Some(player) = children
        .iter_descendants(ready.entity)
        .find(|entity| players.contains(*entity))
    else {
        error!("Container clips {:?} have no player", container.clips);
        return;
    };
    let (graph, nodes) = AnimationGraph::from_clips(handles.iter().cloned());
    commands
        .entity(player)
        .insert(AnimationGraphHandle(graphs.add(graph)));
    if state.open {
        if let Ok(mut player) = players.get_mut(player) {
            for (&node, handle) in nodes.iter().zip(&handles) {
                if let Some(clip) = clips.get(handle) {
                    player.play(node).seek_to(clip.duration());
                }
            }
        }
    }
    commands
        .entity(element.parent())
        .insert(ContainerClips { player, nodes });
}

/// Flips a container and plays its clips from wherever they have got to: forward to open, backward
/// to close. A clip that has run its course holds its last pose.
pub(crate) fn toggle_container(
    toggle: On<ContainerToggle>,
    mut containers: Query<(&mut ContainerState, Option<&ContainerClips>)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok((mut state, clips)) = containers.get_mut(toggle.entity) else {
        return;
    };
    state.open = !state.open;
    let Some(clips) = clips else {
        return;
    };
    let Ok(mut player) = players.get_mut(clips.player) else {
        return;
    };
    for &node in &clips.nodes {
        let active = player.play(node);
        let time = active.seek_time().max(0.);
        // A finished animation no longer advances; `replay` makes it run again.
        active.replay();
        active
            .seek_to(time)
            .set_speed(if state.open { 1. } else { -1. });
    }
}
