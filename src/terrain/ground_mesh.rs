use super::{
    material::TerrainMaterials,
    parcel::{Parcel, ParcelStatus},
    square::SquareArray,
    terrain_shapes::{TerrainShapes, TerrainShapesResource},
    PARCEL_MESH_RESOLUTION, PARCEL_MESH_SCALE, PARCEL_MESH_STRIDE, PARCEL_MESH_VERTEX_COUNT,
    PARCEL_SIZE_F,
};
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

#[derive(Component)]
pub struct ComputeGroundMeshTask(Task<Mesh>);

/// Spawns a task for each parcel to compute the ground mesh geometry.
pub fn compute_ground_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel)>,
    asset_server: Res<AssetServer>,
    terrain_shapes: Res<TerrainShapes>,
    shape_table: Res<Assets<TerrainShapesResource>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, mut parcel) in query.iter_mut() {
        match parcel.status {
            // Spawn task to compute mesh
            ParcelStatus::New | ParcelStatus::Waiting => {
                parcel.status = ParcelStatus::Building;
                let task = pool.spawn(async move {
                    // `ihm` stands for 'Interpolated height map.'
                    let mut ihm = SquareArray::<f32>::new((PARCEL_MESH_STRIDE + 2) as usize, 0, 0.);

                    // Set a test pattern.
                    for x in 5..15 {
                        for z in 5..15 {
                            ihm.set(x, z, 1.);
                        }
                    }

                    // `shm` stands for 'Smoothed height map`
                    let mut shm = SquareArray::<f32>::new(PARCEL_MESH_STRIDE as usize, 0, 0.);

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

                    // const center = this.getRotationalInterpolator(AdjacentIndex.Center);

                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    let mut position: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
                    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
                    let mut indices: Vec<u32> =
                        Vec::with_capacity((PARCEL_MESH_RESOLUTION.pow(2)) as usize);
                    // let terrain_shapes_handle: Handle<TerrainShapesResource> =
                    //     asset_server.load("terrain.tsh.msgpack");

                    // let terrain_shapes = shape_table.get(&terrain_shapes.0);

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