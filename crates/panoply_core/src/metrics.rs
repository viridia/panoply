//! Grid sizes shared by terrain and scenery. A realm is divided into two grids: terrain parcels
//! (see `panoply_terrain::metrics`) and the larger scenery precincts defined here.

/// Width and depth of a scenery precinct, in meters.
pub const PRECINCT_SIZE: i32 = 64;
/// [`PRECINCT_SIZE`] as a float.
pub const PRECINCT_SIZE_F: f32 = PRECINCT_SIZE as f32;
