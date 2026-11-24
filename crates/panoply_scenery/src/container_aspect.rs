use bevy::prelude::*;
use panoply_exemplar::{annotations::AspectIcon, *};
use serde::{Deserialize, Serialize};

/// A fixture or wall tile that opens and closes, such as a chest or a sarcophagus. How it looks
/// doing so is its `SceneryAnimations`, on the input `open`.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/package-open.png"))]
pub struct Container {}

impl NativeAspect for Container {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Fixture || kind == InstanceKind::Wall
    }

    fn attach_state(&self, entity: &mut EntityWorldMut) {
        entity.insert_if_new(ContainerState::default());
    }

    /// Its contents.
    fn has_state(&self) -> bool {
        true
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

/// Opens a closed container, or closes an open one.
#[derive(EntityEvent)]
pub struct ContainerToggle {
    /// The container.
    pub entity: Entity,
}

pub(crate) fn toggle_container(
    toggle: On<ContainerToggle>,
    mut containers: Query<&mut ContainerState>,
) {
    if let Ok(mut state) = containers.get_mut(toggle.entity) {
        state.open = !state.open;
    }
}
