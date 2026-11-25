//! Natives that need nothing of Panoply's, registered by [`SagaPlugin`](crate::SagaPlugin).

use bevy::prelude::*;

use crate::{ExprType, HostParam, HostState, NativeMemo, TrackingScope, VMError};

/// Registers the natives in this module.
pub fn add_natives(host: &mut HostState) {
    let param = |name: &str, ty: ExprType| HostParam {
        name: name.into(),
        ty,
    };
    host.add_memo::<Proximity>(
        "proximity",
        vec![
            param("subject", ExprType::Entity),
            param("object", ExprType::Entity),
            param("enter", ExprType::F32),
            param("leave", ExprType::F32),
        ],
        ExprType::Boolean,
    );
}

/// `proximity(subject, object, enter, leave)`: whether two entities are near, turning true
/// within `enter` meters and false only beyond `leave`, so that one at the edge does not flicker.
/// False while either has no position.
#[derive(Default)]
pub struct Proximity {
    near: bool,
}

impl NativeMemo for Proximity {
    type Args = (Entity, Entity, f32, f32);
    type Value = bool;

    fn compute(
        &mut self,
        (subject, object, enter, leave): Self::Args,
        world: &World,
        tracking: &mut TrackingScope,
    ) -> Result<bool, VMError> {
        let subject = position(world, subject, tracking);
        let object = position(world, object, tracking);
        let (Some(subject), Some(object)) = (subject, object) else {
            self.near = false;
            return Ok(false);
        };
        let distance = subject.distance(object);
        self.near = if self.near {
            distance <= leave
        } else {
            distance < enter
        };
        Ok(self.near)
    }
}

/// Where `entity` is in the world, its read tracked.
fn position(world: &World, entity: Entity, tracking: &mut TrackingScope) -> Option<Vec3> {
    let transform = world
        .get_entity(entity)
        .ok()
        .and_then(|entity| entity.get::<GlobalTransform>());
    tracking.track_component::<GlobalTransform>(entity, world, transform.is_some());
    transform.map(GlobalTransform::translation)
}
