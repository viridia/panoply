use super::{
    parcel::{Parcel, ParcelStatus},
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
pub struct ParcelComputeTask(Task<Mesh>);

pub fn build_parcels(mut commands: Commands, mut query: Query<(Entity, &mut Parcel)>) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, mut parcel) in query.iter_mut() {
        match parcel.status {
            // Spawn task to compute mesh
            ParcelStatus::New | ParcelStatus::Waiting => {
                parcel.status = ParcelStatus::Building;
                let task = pool.spawn(async move {
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    let mut position: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
                    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
                    let mut indices: Vec<u32> =
                        Vec::with_capacity((PARCEL_MESH_RESOLUTION.pow(2)) as usize);

                    // Generate vertices
                    for z in 0..PARCEL_MESH_STRIDE {
                        for x in 0..PARCEL_MESH_STRIDE {
                            position.push([
                                x as f32 * PARCEL_MESH_SCALE,
                                0.,
                                z as f32 * PARCEL_MESH_SCALE,
                            ]);
                            normal.push([0., 1., 0.])
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
                commands.entity(entity).insert(ParcelComputeTask(task));
            }

            _ => {}
        }
    }
}

pub fn apply_build_parcels(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel, &mut ParcelComputeTask)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Reset the visibility bits for all parcels.
    for (entity, mut parcel, mut task) in query.iter_mut() {
        if let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) {
            // Add our new PbrBundle of components to our tagged entity
            commands.entity(entity).insert(PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(Color::GREEN.into()),
                transform: Transform::from_xyz(
                    parcel.coords.x as f32 * PARCEL_SIZE_F,
                    2.0,
                    parcel.coords.y as f32 * PARCEL_SIZE_F,
                ),
                visibility: Visibility::Visible,
                ..default()
            });
            parcel.status = ParcelStatus::Ready;
            commands.entity(entity).remove::<ParcelComputeTask>();
        }
    }
}
