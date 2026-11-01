//! The precinct file format: one asset per 64 m area holding everything authored there but
//! terrain (D24). Scenery and actors each spawn their part of it.

// REFERENCE(PAN3l): the precinct's actor list, compiled when precincts carry actors.
// mod actor_instance;
mod floor_region;
mod precinct_asset;

pub use floor_region::FloorRegionSer;
pub use precinct_asset::*;
