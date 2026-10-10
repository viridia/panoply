pub mod layers;
pub mod metrics;
pub mod random;
pub mod realm;
pub mod realm_physics;
pub mod transition;
pub mod viewpoint;

use bevy::{
    app::{App, FixedUpdate, Plugin, PostUpdate, Update},
    asset::AssetApp,
    prelude::Component,
};
pub use layers::ReservedLayers;
pub use realm::Realm;
use realm::{load_realms, RealmData, RealmsHandleResource, RealmsLoader};
use realm_physics::realm_physics_system;
pub use realm_physics::RealmPhysics;
use viewpoint::{fit_shadow_cascades, goto_location, viewpoint_transitions};
pub use viewpoint::{GotoLocation, Viewpoint};

/// Marker which identifies the primary camera.
#[derive(Component)]
pub struct PrimaryCamera;

/// Marker which identifies the HUD camera.
#[derive(Component)]
pub struct HudCamera;

pub struct PanoplyCorePlugin;

impl Plugin for PanoplyCorePlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(RealmsLoader)
            .init_asset::<RealmData>()
            .init_resource::<RealmsHandleResource>()
            .add_message::<GotoLocation>()
            .add_systems(Update, (load_realms, viewpoint_transitions, goto_location))
            .add_systems(
                PostUpdate,
                (viewpoint::update_camera_pos, fit_shadow_cascades),
            )
            .add_systems(FixedUpdate, realm_physics_system);
    }
}
