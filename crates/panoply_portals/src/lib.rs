//! Portals: an aperture in a wall or fixture through which another place is seen, and crossed.
//! For now, only the aspects that define them, so that precincts and catalogs keep them.

use bevy::prelude::*;

mod portal_aspect;

pub use portal_aspect::{Portal, PortalSide, PortalTarget};

pub struct PanoplyPortalsPlugin;

impl Plugin for PanoplyPortalsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Portal>()
            .register_type::<PortalSide>()
            .register_type::<PortalTarget>();
    }
}
