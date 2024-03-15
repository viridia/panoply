use bevy::{
    app::{App, Plugin, Update},
    ecs::schedule::IntoSystemConfigs,
    prelude::*,
};

use self::{
    active_portal::{spawn_portals, update_portals},
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
            .add_systems(Startup, update_config)
            .add_systems(Update, (spawn_portals, update_portals.after(spawn_portals)));
    }
}

fn update_config(mut config_store: ResMut<GizmoConfigStore>) {
    for (_, config, _) in config_store.iter_mut() {
        config.depth_bias = -1.;
    }
}
