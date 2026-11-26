//! The precinct file format: one asset per 64 m area holding everything authored there but
//! terrain (D24). Scenery and actors each spawn their part of it.

mod actor_instance;
mod floor_region;
mod instance_key;
mod precinct_asset;
mod precinct_index;

pub use actor_instance::ActorInstanceData;
pub use floor_region::FloorRegionSer;
pub use instance_key::{InstanceKey, KeyedInstances};
pub use precinct_asset::*;
pub use precinct_index::*;
