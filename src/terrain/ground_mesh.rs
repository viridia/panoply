use super::{
    material::TerrainMaterials,
    parcel::{Parcel, ParcelStatus},
    square::{RotatingSquareArray, SquareArray},
    terrain_shapes::{TerrainShapesAsset, TerrainShapesHandle},
    PARCEL_MESH_RESOLUTION, PARCEL_MESH_SCALE, PARCEL_MESH_STRIDE, PARCEL_MESH_VERTEX_COUNT,
    PARCEL_SIZE, PARCEL_SIZE_F,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

#[derive(Component)]
pub struct ComputeGroundMeshTask(Task<Mesh>);

pub const HEIGHT_SCALE: f32 = 0.5;

/// Spawns a task for each parcel to compute the ground mesh geometry.
pub fn compute_ground_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel)>,
    server: Res<AssetServer>,
    ts_handle: Res<TerrainShapesHandle>,
    ts_assets: Res<Assets<TerrainShapesAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, mut parcel) in query.iter_mut() {
        match parcel.status {
            // Spawn task to compute mesh
            ParcelStatus::New | ParcelStatus::Waiting => {
                let handle = ts_handle.0.clone();
                if server.get_load_state(&handle) != LoadState::Loaded {
                    return;
                }

                let shapes = ts_assets.get(&handle).expect("msg").0.clone();
                parcel.status = ParcelStatus::Building;
                let task = pool.spawn(async move {
                    // `ihm` stands for 'Interpolated height map.'
                    let mut ihm = SquareArray::<f32>::new((PARCEL_MESH_STRIDE + 2) as usize, 0.);

                    let shapes_table = shapes.lock().unwrap();

                    // Add terrain heights for center plot and all eight neighbors
                    for z in &[-1, 0, 1] {
                        for x in &[-1, 0, 1] {
                            let shape = shapes_table.get(10);
                            accumulate(
                                &shape.height,
                                &mut ihm,
                                1 + x * PARCEL_MESH_RESOLUTION,
                                1 + z * PARCEL_MESH_RESOLUTION,
                                0,
                            );
                        }
                    }

                    // Now re-scale the rows and columns with more than one accumulated value.
                    // Note that some cells will be visited twice, which is what we want.
                    for i in 0..PARCEL_MESH_RESOLUTION + 3 {
                        *ihm.get_mut_ref(1, i) *= 0.5;
                        *ihm.get_mut_ref(PARCEL_MESH_RESOLUTION + 1, i) *= 0.5;
                        *ihm.get_mut_ref(i, 1) *= 0.5;
                        *ihm.get_mut_ref(i, PARCEL_MESH_RESOLUTION + 1) *= 0.5;
                    }

                    // `shm` stands for 'Smoothed height map`
                    let mut shm = SquareArray::<f32>::new(PARCEL_MESH_STRIDE as usize, 0.);

                    // Compute smoothed mesh
                    for z in 0..PARCEL_MESH_STRIDE {
                        for x in 0..PARCEL_MESH_STRIDE {
                            let h4 = ihm.get(x, z + 1)
                                + ihm.get(x + 2, z + 1)
                                + ihm.get(x + 1, z)
                                + ihm.get(x + 1, z + 2);

                            shm.set(x, z, h4 * 0.25);
                            // shm[dstIndex] = h4 * 0.25 + hOffset[dstIndex];
                        }
                    }

                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    let mut position: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
                    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
                    let mut indices: Vec<u32> =
                        Vec::with_capacity((PARCEL_MESH_RESOLUTION.pow(2)) as usize);

                    // Generate vertices
                    let mut n = Vec3::new(0., 1., 0.);
                    for z in 0..PARCEL_MESH_STRIDE {
                        for x in 0..PARCEL_MESH_STRIDE {
                            position.push([
                                x as f32 * PARCEL_MESH_SCALE,
                                shm.get(x, z),
                                z as f32 * PARCEL_MESH_SCALE,
                            ]);

                            // Off the edge of the smoothing array, use unsmoothed
                            if x == 0 {
                                n.x = ihm.get(x, z + 1) - shm.get(x + 1, z);
                            } else if x == PARCEL_MESH_STRIDE - 1 {
                                n.x = shm.get(x - 1, z) - ihm.get(x + 2, z + 1);
                            } else {
                                n.x = shm.get(x - 1, z) - shm.get(x + 1, z);
                            }

                            if z == 0 {
                                n.z = ihm.get(x + 1, z) - shm.get(x, z + 1);
                            } else if z == PARCEL_MESH_STRIDE - 1 {
                                n.z = shm.get(x, z - 1) - ihm.get(x + 1, z + 2);
                            } else {
                                n.z = shm.get(x, z - 1) - shm.get(x, z + 1);
                            }

                            n.y = 1.;
                            normal.push(n.normalize().to_array())
                        }
                    }

                    // Generate indices
                    for z in 0..PARCEL_MESH_STRIDE - 1 {
                        for x in 0..PARCEL_MESH_STRIDE - 1 {
                            // Handle terrain holes.
                            // if (isHole && isHole(x, y)) {
                            //     continue;
                            //   }
                            let a = (z * PARCEL_MESH_STRIDE + (x + 1)) as u32;
                            let b = (z * PARCEL_MESH_STRIDE + x) as u32;
                            let c = ((z + 1) * PARCEL_MESH_STRIDE + x) as u32;
                            let d = ((z + 1) * PARCEL_MESH_STRIDE + (x + 1)) as u32;

                            indices.push(a);
                            indices.push(b);
                            indices.push(d);
                            indices.push(b);
                            indices.push(c);
                            indices.push(d);
                        }
                    }

                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
                    mesh.set_indices(Some(Indices::U32(indices)));
                    mesh.compute_aabb();
                    mesh
                });

                commands.entity(entity).insert(ComputeGroundMeshTask(task));
            }

            _ => {}
        }
    }
}

/// Consumes the output of the compute task and creates a mesh component for the ground geometry.
pub fn insert_ground_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel, &mut ComputeGroundMeshTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain_materials: ResMut<TerrainMaterials>,
) {
    // Reset the visibility bits for all parcels.
    for (entity, mut parcel, mut task) in query.iter_mut() {
        if let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) {
            // Add our new PbrBundle of components to our tagged entity
            commands.entity(entity).insert(MaterialMeshBundle {
                mesh: meshes.add(mesh),
                material: terrain_materials.ground.clone(),
                transform: Transform::from_xyz(
                    parcel.coords.x as f32 * PARCEL_SIZE_F,
                    2.0,
                    parcel.coords.y as f32 * PARCEL_SIZE_F,
                ),
                visibility: Visibility::Visible,
                ..default()
            });
            parcel.status = ParcelStatus::Ready;
            commands.entity(entity).remove::<ComputeGroundMeshTask>();
        }
    }
}

fn accumulate(
    src: &SquareArray<i8>,
    dst: &mut SquareArray<f32>,
    x_offset: i32,
    z_offset: i32,
    rotation: i32,
) {
    let src_rot = RotatingSquareArray::new(src.size(), rotation, &src.elts());
    let x0 = x_offset.max(0);
    let x1 = (x_offset + PARCEL_MESH_RESOLUTION + 1).min(dst.size() as i32);
    let z0 = z_offset.max(0);
    let z1 = (z_offset + PARCEL_MESH_RESOLUTION + 1).min(dst.size() as i32);

    for z in z0..z1 {
        for x in x0..x1 {
            *dst.get_mut_ref(x, z) += interpolate_square(
                &src_rot,
                (x - x_offset) as f32 * PARCEL_MESH_SCALE,
                (z - z_offset) as f32 * PARCEL_MESH_SCALE,
            ) * HEIGHT_SCALE;
        }
    }
}

/// Returns a callable object that computes the interpolated terrain height for any point
/// on the terrain plot. Note that this is before smoothing, since that happens at
/// the terrain parcel level.
fn interpolate_square(square: &RotatingSquareArray<i8>, x: f32, z: f32) -> f32 {
    // Get interpolated height - note doesn't incorporate smoothing.
    let cx = x.clamp(0., PARCEL_SIZE as f32);
    let cz = z.clamp(0., PARCEL_SIZE as f32);
    let x0 = cx.floor();
    let x1 = cx.ceil();
    let z0 = cz.floor();
    let z1 = cz.ceil();

    let h00 = square.get(x0 as i32, z0 as i32) as f32;
    let h01 = square.get(x0 as i32, z1 as i32) as f32;
    let h10 = square.get(x1 as i32, z0 as i32) as f32;
    let h11 = square.get(x1 as i32, z1 as i32) as f32;

    let fx = cx - x0;
    let fy = cz - z0;
    let h0 = h00 * (1. - fx) + h10 * fx;
    let h1 = h01 * (1. - fx) + h11 * fx;
    return h0 * (1. - fy) + h1 * fy;
}
