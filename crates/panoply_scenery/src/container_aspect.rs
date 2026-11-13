use bevy::prelude::*;
use panoply_exemplar::{annotations::AspectIcon, *};
use serde::{Deserialize, Serialize};

use super::{FIXTURE_TYPE, WALL_TYPE};

/// A fixture or wall tile that opens and closes, such as a chest or a sarcophagus. How it looks
/// doing so is its `SceneryAnimations`, on the input `open`.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/package-open.png"))]
pub struct Container {}

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
