mod biome;
mod ground_material;
pub mod metrics;
mod parcel;
// mod parcel_cache;
// mod compute_mesh;
mod plugin;
mod rotator;
mod square;
mod terrain_contours;
mod terrain_fx;
mod terrain_groups;
mod water_material;
// mod water_mesh;
// mod terrain_map;

pub use biome::*;
pub use ground_material::{GroundMaterial, GroundMaterialCache};
pub use metrics::*;
pub use parcel::*;
pub use rotator::*;
pub use square::*;
pub use terrain_contours::*;
pub use terrain_fx::*;
pub use terrain_groups::*;
pub use water_material::{WaterMaterial, WaterMaterialResource, ATTRIBUTE_DEPTH_MOTION};

pub use plugin::PanoplyTerrainPlugin;

// embedded_asset!(app, "assets/shaders/gradient_rect.wgsl");
