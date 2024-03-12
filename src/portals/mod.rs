use bevy::app::{App, Plugin};

use self::portal_aspect::{Portal, PortalSide, PortalTarget};

mod portal_aspect;

pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Portal>()
            .register_type::<PortalTarget>()
            .register_type::<PortalSide>();
    }
}
