mod parcel;
mod parcelcache;
mod terrainshape;

pub const PLOT_LENGTH: i32 = 16;
pub const PLOT_LENGTH_F: f32 = PLOT_LENGTH as f32;

pub use parcelcache::spawn_parcels;
pub use parcelcache::ParcelCache;
