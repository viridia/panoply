mod hex_color;
pub mod layers;
pub mod metrics;
pub mod random;
pub mod realm;
pub mod realm_physics;
pub mod sunlight;
pub mod transition;
pub mod viewpoint;
pub mod viewport;

use bevy::{
    app::{App, FixedUpdate, HierarchyPropagatePlugin, Plugin, PostUpdate, PropagateSet, Update},
    asset::AssetApp,
    camera::visibility::{RenderLayers, VisibilitySystems},
    prelude::{Component, IntoScheduleConfigs},
};
pub use hex_color::HexColor;
pub use layers::ReservedLayers;
pub use realm::Realm;
use realm::{load_realms, RealmData, RealmsHandleResource, RealmsLoader};
use realm_physics::realm_physics_system;
pub use realm_physics::{OwnedColliders, RealmPhysics};
pub use sunlight::Sunlight;
use viewpoint::{fit_shadow_cascades, goto_location, viewpoint_transitions};
pub use viewpoint::{GotoLocation, Viewpoint};
pub use viewport::{ViewOverlay, ViewportInset};

/// Marker which identifies the primary camera.
#[derive(Component)]
pub struct PrimaryCamera;

/// Marker which identifies the HUD camera.
#[derive(Component)]
pub struct HudCamera;

pub struct PanoplyCorePlugin;

impl Plugin for PanoplyCorePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HexColor>()
            .register_type::<Option<HexColor>>()
            .register_asset_loader(RealmsLoader)
            .init_asset::<RealmData>()
            .init_resource::<RealmsHandleResource>()
            .init_resource::<Sunlight>()
            .init_resource::<ViewportInset>()
            .add_message::<GotoLocation>()
            .add_systems(Update, (load_realms, viewpoint_transitions, goto_location))
            .add_systems(
                PostUpdate,
                (
                    viewpoint::update_camera_pos,
                    fit_shadow_cascades,
                    sunlight::sync_sunlight,
                ),
            )
            .add_systems(FixedUpdate, realm_physics_system)
            // A scene instance (flora, scenery, model) carries `Propagate<RenderLayers>` with its
            // realm's layer, which reaches every descendant, including ones spawned later, such
            // as outline children. Before visibility, so a new scene never draws on layer 0.
            .add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PostUpdate))
            .configure_sets(
                PostUpdate,
                PropagateSet::<RenderLayers>::default().before(VisibilitySystems::CheckVisibility),
            );
    }
}
