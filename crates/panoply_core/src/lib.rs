pub mod layers;
pub mod random;
pub mod realm;
pub mod realm_physics;
pub mod viewpoint;

use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    asset::AssetApp,
};
pub use layers::ReservedLayers;
pub use realm::Realm;
use realm::{load_realms, RealmData, RealmsHandleResource, RealmsLoader};
use realm_physics::realm_physics_system;
pub use realm_physics::RealmPhysics;
pub use viewpoint::Viewpoint;

pub struct PanoplyCorePlugin;

impl Plugin for PanoplyCorePlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(RealmsLoader)
            .init_asset::<RealmData>()
            .init_resource::<RealmsHandleResource>()
            .add_systems(Update, load_realms)
            .add_systems(FixedUpdate, realm_physics_system);
    }
}
