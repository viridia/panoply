//! Portals: an aperture in a wall or fixture through which another place is seen, and later
//! crossed (PAN4c).

use bevy::{
    asset::{embedded_asset, AssetEventSystems},
    camera::CameraUpdateSystems,
    prelude::*,
};

mod aperture_material;
mod portal_aspect;
mod portal_projection;
mod portal_view;

use aperture_material::ApertureMaterial;
pub use portal_aspect::{Portal, PortalSide, PortalTarget};
pub use portal_projection::PortalProjection;
pub use portal_view::{aperture_transform, PortalAperture, PortalCamera, PortalView};

pub struct PanoplyPortalsPlugin;

impl Plugin for PanoplyPortalsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/aperture.wesl");
        app.register_type::<Portal>()
            .register_type::<PortalSide>()
            .register_type::<PortalTarget>()
            .add_plugins(MaterialPlugin::<ApertureMaterial>::default())
            .add_systems(
                PostUpdate,
                portal_view::update_portal_views
                    .after(panoply_core::viewpoint::update_camera_pos)
                    .before(TransformSystems::Propagate)
                    // The portal camera's projection changes as the primary camera moves; Bevy
                    // turns it into the camera's matrix in `CameraUpdateSystems`, and the
                    // aperture's material change reaches the render world in `AssetEventSystems`.
                    // Before both, the cut, the image's place and the camera agree in each frame.
                    .before(CameraUpdateSystems)
                    .before(AssetEventSystems),
            );
    }
}
