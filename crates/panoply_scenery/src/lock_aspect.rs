use bevy::prelude::*;
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};

use super::{FIXTURE_TYPE, WALL_TYPE};

/// A fixture or wall tile that can be locked, such as a chest. Whether it starts locked; an
/// instance that does overrides its exemplar's with its own.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Lock {
    /// True if an instance starts locked.
    pub locked: bool,
}

impl Aspect for Lock {
    fn name(&self) -> &str {
        "Lock"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == FIXTURE_TYPE || meta_type == WALL_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Lock> = RemoveComponent::<Lock>::new();
        entity.insert(self.clone());
        entity.insert_if_new(LockState {
            locked: self.locked,
        });
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
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
