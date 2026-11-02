//! The precinct file format: one asset per 64 m area holding everything authored there but
//! terrain (D24). Scenery and actors each spawn their part of it.

mod actor_instance;
mod floor_region;
mod precinct_asset;

pub use actor_instance::ActorInstanceData;
pub use floor_region::FloorRegionSer;
pub use precinct_asset::*;
