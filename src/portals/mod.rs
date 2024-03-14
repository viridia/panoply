use bevy::app::{App, Plugin, Update};

use self::{
    active_portal::spawn_portals,
    portal_aspect::{Portal, PortalSide, PortalTarget},
};

mod active_portal;
mod portal_aspect;

pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Portal>()
            .register_type::<PortalTarget>()
            .register_type::<PortalSide>()
            .add_systems(Update, spawn_portals);
    }
}
