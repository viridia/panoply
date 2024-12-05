use crate::{
    RotatingSquareArray, ShapeRef, SquareArray, TerrainContoursTable, ADJACENT_COUNT,
    PARCEL_HEIGHT_SCALE, PARCEL_MESH_SCALE, PARCEL_MESH_SIZE, PARCEL_MESH_SIZE_U,
    PARCEL_MESH_STRIDE, PARCEL_MESH_STRIDE_U, PARCEL_SIZE_F,
};

pub fn compute_interpolated_mesh(
    // `ihm` stands for 'Interpolated height map.'
    ihm: &mut SquareArray<f32>,
    shape_refs: [ShapeRef; ADJACENT_COUNT],
    shapes_table: &TerrainContoursTable,
) {
    let mut weights = SquareArray::<f32>::new(PARCEL_MESH_STRIDE_U + 2, 0.);

    let center = shapes_table.get(shape_refs[4].shape as usize);
    if !center.has_terrain {
        println!("No terrain");
    }

    // Add terrain heights for center plot and all eight neighbors
    for z in &[-1, 0, 1] {
        for x in &[-1, 0, 1] {
            let shape_ref = shape_refs[(z * 3 + x + 4) as usize];
            let shape = shapes_table.get(shape_ref.shape as usize);
            if shape.has_terrain {
                accumulate(
                    &shape.height,
                    ihm,
                    &mut weights,
                    1 + x * PARCEL_MESH_SIZE,
                    1 + z * PARCEL_MESH_SIZE,
                    shape_ref.rotation as i32,
                );
            }
        }
    }

    for z in 0..=PARCEL_MESH_SIZE_U + 2 {
        for x in 0..=PARCEL_MESH_SIZE_U + 2 {
            let w = weights.get(x, z);
            if w > 0. {
                *ihm.get_mut_ref(x, z) /= w;
            }
        }
    }
}

pub fn compute_smoothed_mesh(shm: &mut SquareArray<f32>, ihm: &SquareArray<f32>) {
    // Compute smoothed mesh
    for z in 0..PARCEL_MESH_STRIDE_U {
        for x in 0..PARCEL_MESH_STRIDE_U {
            let h4 = ihm.get(x, z + 1)
                + ihm.get(x + 2, z + 1)
                + ihm.get(x + 1, z)
                + ihm.get(x + 1, z + 2);

            shm.set(x, z, h4 * 0.25);
            // shm[dstIndex] = h4 * 0.25 + hOffset[dstIndex];
        }
    }
}

fn accumulate(
    src: &SquareArray<i8>,
    dst: &mut SquareArray<f32>,
    weight: &mut SquareArray<f32>,
    x_offset: i32,
    z_offset: i32,
    rotation: i32,
) {
    let src_rot = RotatingSquareArray::new(src.size(), rotation, src.elts());
    let x0 = x_offset.max(0);
    let x1 = (x_offset + PARCEL_MESH_SIZE + 1).min(dst.size() as i32);
    let z0 = z_offset.max(0);
    let z1 = (z_offset + PARCEL_MESH_SIZE + 1).min(dst.size() as i32);

    for z in z0..z1 {
        for x in x0..x1 {
            *dst.get_mut_ref(x as usize, z as usize) += interpolate_square(
                &src_rot,
                (x - x_offset) as f32 * PARCEL_MESH_SCALE,
                (z - z_offset) as f32 * PARCEL_MESH_SCALE,
            ) * PARCEL_HEIGHT_SCALE;
            *weight.get_mut_ref(x as usize, z as usize) += 1.0;
        }
    }
}

/// Returns a callable object that computes the interpolated terrain height for any point
/// on the terrain plot. Note that this is before smoothing, since that happens at
/// the terrain parcel level.
fn interpolate_square(square: &RotatingSquareArray<i8>, x: f32, z: f32) -> f32 {
    // Get interpolated height - note doesn't incorporate smoothing.
    let cx = x.clamp(0., PARCEL_SIZE_F);
    let cz = z.clamp(0., PARCEL_SIZE_F);
    let x0 = cx.floor();
    let x1 = cx.ceil();
    let z0 = cz.floor();
    let z1 = cz.ceil();

    let h00 = square.get(x0 as usize, z0 as usize) as f32;
    let h01 = square.get(x0 as usize, z1 as usize) as f32;
    let h10 = square.get(x1 as usize, z0 as usize) as f32;
    let h11 = square.get(x1 as usize, z1 as usize) as f32;

    let fx = cx - x0;
    let fy = cz - z0;
    let h0 = h00 * (1. - fx) + h10 * fx;
    let h1 = h01 * (1. - fx) + h11 * fx;
    h0 * (1. - fy) + h1 * fy
}
