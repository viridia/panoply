//! Colliders for generated trees: a capsule per trunk, which blocks walking, and a canopy over
//! the forest, which catches what falls onto it. Trunks can be 0.4 m apart and actors are 0.6 m
//! wide, so without the canopy an actor dropped onto a forest could land between trunks in a
//! pocket it cannot leave.
//!
//! The canopy is quantized to the terrain grid: it covers every tree cell, and every empty cell
//! that a flood fill from outside does not reach (an enclosed clearing). Being overhead, it never
//! blocks walking, so covering more than needed costs nothing. No generated tree shares a cell
//! with a building, and cells with terrain effects are never covered, so it never cuts through
//! a floor.

use bevy::prelude::*;
use rapier3d::prelude::{Pose, SharedShape, TriMeshFlags};

use crate::{
    FloraType, ParcelTerrainFx, RotatingSquareArray, ShapeRef, TerrainContoursTable,
    ADJACENT_COUNT, CENTER_SHAPE, PARCEL_HEIGHT_SCALE, PARCEL_SIZE, PARCEL_SIZE_U,
};

/// Height of trunk colliders and of the canopy above the ground: over the 1.4 m actor.
pub const CANOPY_HEIGHT: f32 = 2.0;

/// How far past the parcel's edge, in cells, the flood fill looks for a way out, so that a
/// clearing enclosed across a parcel edge is covered too. Larger enclosures count as open.
const ENCLOSURE_MARGIN: i32 = 4;

/// The shapes of a parcel's trees, in parcel coordinates.
pub struct FloraShapes {
    /// One compound of trunk capsules; `None` without trees.
    pub trunks: Option<SharedShape>,
    /// `None` without covered cells.
    pub canopy: Option<SharedShape>,
}

/// Builds the trunk compound from (base position, radius) pairs.
pub fn trunk_shape(trunks: &[(Vec3, f32)]) -> Option<SharedShape> {
    if trunks.is_empty() {
        return None;
    }
    let parts = trunks
        .iter()
        .map(|&(base, radius)| {
            let radius = radius.min(CANOPY_HEIGHT * 0.5);
            let center = base + Vec3::Y * CANOPY_HEIGHT * 0.5;
            (
                Pose::from_translation(center),
                SharedShape::capsule_y(CANOPY_HEIGHT * 0.5 - radius, radius),
            )
        })
        .collect();
    Some(SharedShape::compound(parts))
}

/// Builds the canopy of the parcel whose nine adjacent shapes are `contours` (the center one
/// is the parcel's own).
pub fn canopy_shape(
    contours: &[ShapeRef; ADJACENT_COUNT],
    table: &TerrainContoursTable,
    terrain_fx: &ParcelTerrainFx,
) -> Option<SharedShape> {
    let flora: Vec<_> = contours
        .iter()
        .map(|shape_ref| {
            let contour = table.get(shape_ref.shape as usize);
            contour
                .has_terrain
                .then(|| contour.into_flora_square(shape_ref.rotation))
        })
        .collect();
    let has_fx = |x: i32, z: i32| {
        let known = (-1..=PARCEL_SIZE).contains(&x) && (-1..=PARCEL_SIZE).contains(&z);
        known
            && !terrain_fx
                .get((x + 1) as usize, (z + 1) as usize)
                .effect
                .is_empty()
    };
    // Past the effects' one-cell border, a neighbour's flora map alone decides.
    let is_tree = |x: i32, z: i32| {
        let (px, pz) = (x.div_euclid(PARCEL_SIZE), z.div_euclid(PARCEL_SIZE));
        let Some(square) = &flora[((pz + 1) * 3 + px + 1) as usize] else {
            return false;
        };
        let (cx, cz) = (x.rem_euclid(PARCEL_SIZE), z.rem_euclid(PARCEL_SIZE));
        square.get(cx as usize, cz as usize) == FloraType::RandomTree && !has_fx(x, z)
    };
    let cover = canopy_cover(is_tree, has_fx);

    let center = &contours[CENTER_SHAPE];
    let contour = table.get(center.shape as usize);
    let heights = RotatingSquareArray::new(
        contour.height.size(),
        center.rotation,
        contour.height.elts(),
    );
    canopy_mesh(&cover, |x, z| {
        heights.get(x, z) as f32 * PARCEL_HEIGHT_SCALE + CANOPY_HEIGHT
    })
}

/// Which of the parcel's cells the canopy covers, row by row (`x + z * PARCEL_SIZE`). Cells are
/// in parcel coordinates and may lie past its edges. A tree cell is a wall to the flood fill,
/// which moves between empty cells by their sides only: both choices can only cover more.
fn canopy_cover(
    is_tree: impl Fn(i32, i32) -> bool,
    has_fx: impl Fn(i32, i32) -> bool,
) -> Vec<bool> {
    const M: i32 = ENCLOSURE_MARGIN;
    const W: i32 = PARCEL_SIZE + 2 * M;
    let index = |x: i32, z: i32| ((x + M) + (z + M) * W) as usize;
    let tree: Vec<bool> = (0..W * W).map(|i| is_tree(i % W - M, i / W - M)).collect();

    let mut reached = vec![false; (W * W) as usize];
    let mut stack: Vec<(i32, i32)> = (-M..PARCEL_SIZE + M)
        .flat_map(|i| {
            [
                (i, -M),
                (i, PARCEL_SIZE + M - 1),
                (-M, i),
                (PARCEL_SIZE + M - 1, i),
            ]
        })
        .collect();
    while let Some((x, z)) = stack.pop() {
        let outside = !(-M..PARCEL_SIZE + M).contains(&x) || !(-M..PARCEL_SIZE + M).contains(&z);
        if outside || reached[index(x, z)] || tree[index(x, z)] {
            continue;
        }
        reached[index(x, z)] = true;
        stack.extend([(x + 1, z), (x - 1, z), (x, z + 1), (x, z - 1)]);
    }

    (0..PARCEL_SIZE * PARCEL_SIZE)
        .map(|i| {
            let (x, z) = (i % PARCEL_SIZE, i / PARCEL_SIZE);
            tree[index(x, z)] || (!reached[index(x, z)] && !has_fx(x, z))
        })
        .collect()
}

/// The covered cells as one trimesh facing up, each corner at `height(x, z)`. Corners are
/// shared, so the surface is continuous: on a slope it slopes, with no steps or holes.
fn canopy_mesh(cover: &[bool], height: impl Fn(usize, usize) -> f32) -> Option<SharedShape> {
    const CORNERS: usize = PARCEL_SIZE_U + 1;
    let mut corner_index = vec![u32::MAX; CORNERS * CORNERS];
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut corner = |x: usize, z: usize, vertices: &mut Vec<Vec3>| {
        let slot = &mut corner_index[x + z * CORNERS];
        if *slot == u32::MAX {
            *slot = vertices.len() as u32;
            vertices.push(Vec3::new(x as f32, height(x, z), z as f32));
        }
        *slot
    };
    let mut indices: Vec<[u32; 3]> = Vec::new();
    for z in 0..PARCEL_SIZE_U {
        for x in 0..PARCEL_SIZE_U {
            if !cover[x + z * PARCEL_SIZE_U] {
                continue;
            }
            let a = corner(x, z, &mut vertices);
            let b = corner(x + 1, z, &mut vertices);
            let c = corner(x + 1, z + 1, &mut vertices);
            let d = corner(x, z + 1, &mut vertices);
            // Wound to face +y.
            indices.extend([[a, c, b], [a, d, c]]);
        }
    }
    if indices.is_empty() {
        return None;
    }
    SharedShape::trimesh_with_flags(vertices, indices, TriMeshFlags::FIX_INTERNAL_EDGES).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover_of(trees: &[(i32, i32)], fx: &[(i32, i32)]) -> Vec<bool> {
        canopy_cover(|x, z| trees.contains(&(x, z)), |x, z| fx.contains(&(x, z)))
    }

    fn covered(cover: &[bool], x: i32, z: i32) -> bool {
        cover[(x + z * PARCEL_SIZE) as usize]
    }

    /// The eight cells around (x, z).
    fn ring(x: i32, z: i32) -> Vec<(i32, i32)> {
        (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| (x + dx, z + dz)))
            .filter(|&cell| cell != (x, z))
            .collect()
    }

    #[test]
    fn covers_trees_and_enclosed_clearings_only() {
        let mut trees = ring(5, 5);
        trees.push((10, 10));
        let cover = cover_of(&trees, &[]);
        assert!(covered(&cover, 4, 4), "tree cell");
        assert!(covered(&cover, 5, 5), "clearing enclosed by trees");
        assert!(covered(&cover, 10, 10), "lone tree");
        assert!(!covered(&cover, 2, 2), "open ground");
        assert_eq!(cover.iter().filter(|c| **c).count(), 10);
    }

    #[test]
    fn a_gap_opens_a_clearing() {
        let trees: Vec<_> = ring(5, 5).into_iter().filter(|&c| c != (5, 4)).collect();
        assert!(!covered(&cover_of(&trees, &[]), 5, 5));
    }

    #[test]
    fn diagonal_gaps_stay_closed() {
        // Trees on the four sides only: the corners are empty but touch the clearing diagonally.
        let trees = [(5, 4), (4, 5), (6, 5), (5, 6)];
        assert!(covered(&cover_of(&trees, &[]), 5, 5));
    }

    #[test]
    fn enclosure_across_the_edge() {
        // A clearing at the parcel's corner (0, 0), closed off by trees in the neighbours.
        let trees = ring(0, 0);
        assert!(covered(&cover_of(&trees, &[]), 0, 0));
    }

    #[test]
    fn effects_are_never_covered() {
        assert!(!covered(&cover_of(&ring(5, 5), &[(5, 5)]), 5, 5));
    }
}
