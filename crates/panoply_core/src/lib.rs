mod fonts;
mod game_mode;
mod hex_color;
pub mod layers;
pub mod metrics;
pub mod nav_point;
mod quit;
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
    prelude::{AppExtStates, Component, IntoScheduleConfigs},
};
pub use fonts::{GAME_UI_FONT, KEY_PROMPT_FONT};
pub use game_mode::GameMode;
use fonts::GameFonts;
pub use hex_color::HexColor;
pub use layers::ReservedLayers;
use nav_point::NavPointsLoader;
pub use nav_point::{NavPoint, NavPointsAsset, NavPointsHandleResource};
pub use quit::{QuitGate, QuitRequest};
pub use realm::Realm;
use realm::{load_realms, RealmData, RealmsHandleResource, RealmsLoader};
use realm_physics::realm_physics_system;
pub use realm_physics::{OwnedColliders, RealmPhysics};
pub use sunlight::Sunlight;
use viewpoint::{fit_shadow_cascades, goto_location, viewpoint_transitions};
pub use viewpoint::{GotoLocation, Viewpoint};
pub use viewport::{GameViewNode, ViewOverlay, ViewportInset};

/// Marker which identifies the primary camera.
#[derive(Component)]
pub struct PrimaryCamera;

/// Marker which identifies the HUD camera.
#[derive(Component)]
pub struct HudCamera;

pub struct PanoplyCorePlugin;

impl Plugin for PanoplyCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameMode>()
            // So plans can read and set the mode.
            .register_type_mutable_state::<GameMode>()
            .register_type::<HexColor>()
            .register_type::<Option<HexColor>>()
            .register_asset_loader(RealmsLoader)
            .init_asset::<RealmData>()
            .init_resource::<RealmsHandleResource>()
            .register_asset_loader(NavPointsLoader)
            .init_asset::<NavPointsAsset>()
            .init_resource::<NavPointsHandleResource>()
            .init_resource::<GameFonts>()
            .init_resource::<Sunlight>()
            .init_resource::<ViewportInset>()
            .init_resource::<QuitGate>()
            .add_message::<QuitRequest>()
            .add_message::<GotoLocation>()
            .add_systems(Update, (load_realms, viewpoint_transitions, goto_location))
            .add_systems(
                PostUpdate,
                (
                    (
                        viewpoint::follow_active_entity,
                        viewpoint::update_camera_pos,
                    )
                        .chain(),
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
