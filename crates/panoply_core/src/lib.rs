pub mod layers;
pub mod random;
pub mod realm;
pub mod realm_physics;
pub mod viewpoint;

use bevy::{
    app::{App, FixedUpdate, Plugin, PostUpdate, Update},
    asset::AssetApp,
    prelude::Component,
    render::camera,
};
pub use layers::ReservedLayers;
pub use realm::Realm;
use realm::{load_realms, RealmData, RealmsHandleResource, RealmsLoader};
use realm_physics::realm_physics_system;
pub use realm_physics::RealmPhysics;
pub use viewpoint::Viewpoint;

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
            .add_systems(Update, load_realms)
            .add_systems(PostUpdate, viewpoint::update_camera_pos)
            .add_systems(FixedUpdate, realm_physics_system);
    }
}
