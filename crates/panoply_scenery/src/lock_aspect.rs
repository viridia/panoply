use bevy::{annotations::DisplayName, prelude::*};
use panoply_exemplar::{annotations::AspectIcon, *};
use serde::{Deserialize, Serialize};

use super::{FIXTURE_TYPE, WALL_TYPE};

/// A fixture or wall tile that can be locked, such as a chest. Whether it starts locked; an
/// instance that does overrides its exemplar's with its own.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/lock.png"))]
#[serde(default)]
pub struct Lock {
    /// True if an instance starts locked.
    #[reflect(@DisplayName("Locked"))]
    pub locked: bool,
}

impl NativeAspect for Lock {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == FIXTURE_TYPE || meta_type == WALL_TYPE
    }

    fn attach_state(&self, entity: &mut EntityWorldMut) {
        entity.insert_if_new(LockState {
            locked: self.locked,
        });
    }

    /// Whether it is locked.
    fn has_state(&self) -> bool {
        true
    }
}

/// Whether a lock is locked now, kept apart from [`Lock`] so that reapplying aspects leaves it as
/// it was. Play state, which a save holds.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Component, Default, Serialize, Deserialize)]
pub struct LockState {
    /// True while locked.
    pub locked: bool,
}
