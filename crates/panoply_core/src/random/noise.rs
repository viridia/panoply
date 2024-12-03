/// Permutation polynomial, 1d
pub fn permute(x: i32) -> i32 {
    ((34 * x + 1) * x).rem_euclid(289)
}

/// Permutation polynomial, 2d
pub fn permute2(x: i32, y: i32) -> i32 {
    permute(x + permute(y))
}

/// Permutation polynomial, 3d
pub fn permute3(x: i32, y: i32, z: i32) -> i32 {
    permute(x + permute(y + permute(z)))
}

/// Spatial noise, integer 1d
pub fn noise(x: i32) -> f32 {
    permute(x) as f32 / 289.0
}

/// Spatial noise, integer 2d
pub fn noise2(x: i32, y: i32) -> f32 {
    permute2(x, y) as f32 / 289.0
}

/// Spatial noise, integer 3d
pub fn noise3(x: i32, y: i32, z: i32) -> f32 {
    permute3(x, y, z) as f32 / 289.0
}
