use std::sync::{Arc, Mutex};

use crate::world::Realm;

use super::{
    parcel::{Parcel, RebuildParcelGroundMesh, ShapeRef, ADJACENT_COUNT, CENTER_SHAPE},
    rotator::RotatingSquareArray,
    square::SquareArray,
    terrain_contours::{TerrainContoursHandle, TerrainContoursTable, TerrainContoursTableAsset},
    terrain_map::TerrainMap,
    TerrainFxVertexAttr, PARCEL_MESH_RESOLUTION, PARCEL_MESH_SCALE, PARCEL_MESH_STRIDE,
    PARCEL_MESH_VERTEX_COUNT, PARCEL_SIZE, PARCEL_TERRAIN_FX_AREA,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_asset::RenderAssetUsages,
        render_resource::{PrimitiveTopology, VertexFormat},
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

pub struct GroundMeshResult {
    mesh: Mesh,
}

#[derive(Component)]
pub struct ComputeGroundMeshTask(Task<Option<GroundMeshResult>>);

pub const HEIGHT_SCALE: f32 = 0.5;

pub const ATTRIBUTE_TERRAIN_FX: MeshVertexAttribute =
    MeshVertexAttribute::new("terrain_fx", 0x1000, VertexFormat::Uint8x4);

/// Spawns a task for each parcel to compute the ground mesh geometry.
pub fn gen_ground_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel), With<RebuildParcelGroundMesh>>,
    realms_query: Query<(&Realm, &TerrainMap)>,
    server: Res<AssetServer>,
    ts_handle: Res<TerrainContoursHandle>,
    ts_assets: Res<Assets<TerrainContoursTableAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, parcel) in query.iter_mut() {
        let realm = realms_query.get(parcel.realm);
        if realm.is_err() {
            return;
        }

        if server.load_state(&ts_handle.0) != LoadState::Loaded {
            return;
        }

        let contours = ts_assets
            .get(&ts_handle.0)
            .expect("asset shapes required")
            .0
            .clone();

        let shape_refs = parcel.contours;
        let terrain_fx = parcel.terrain_fx;
        let task =
            pool.spawn(async move { compute_ground_mesh(shape_refs, terrain_fx, &contours) });
        commands
            .entity(entity)
            .insert(ComputeGroundMeshTask(task))
            .remove::<RebuildParcelGroundMesh>();
    }
}

/// Consumes the output of the compute task and creates a mesh component for the ground geometry.
pub fn insert_ground_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel, &mut ComputeGroundMeshTask)>,
    realms_query: Query<(&Realm, &TerrainMap)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mut parcel, mut task) in query.iter_mut() {
        let realm = realms_query.get(parcel.realm);
        if realm.is_ok() {
            let (_, terrain_map) = realm.unwrap();
            if let Some(task_result) = future::block_on(future::poll_once(&mut task.0)) {
                if let Some(ground_result) = task_result {
                    let ground_mesh = MaterialMeshBundle {
                        mesh: meshes.add(ground_result.mesh),
                        material: terrain_map.ground_material.clone(),
                        visibility: Visibility::Visible,
                        ..default()
                    };
                    match parcel.ground_entity {
                        Some(ground_entity) => {
                            // Replace mesh
                            commands.entity(ground_entity).insert(ground_mesh);
                        }
                        None => {
                            // Insert new mesh entity
                            parcel.ground_entity =
                                Some(commands.spawn(ground_mesh).set_parent(entity).id());
                        }
                    }
                }
                commands.entity(entity).remove::<ComputeGroundMeshTask>();
            }
        }
    }
}

fn compute_ground_mesh(
    shape_refs: [ShapeRef; ADJACENT_COUNT],
    _terrain_fx: [TerrainFxVertexAttr; PARCEL_TERRAIN_FX_AREA],
    shapes: &Arc<Mutex<TerrainContoursTable>>,
) -> Option<GroundMeshResult> {
    let shapes_table = shapes.lock().unwrap();
    let terrain_shape = shapes_table.get(shape_refs[CENTER_SHAPE].shape as usize);
    if !terrain_shape.has_terrain {
        return None;
    }

    // `ihm` stands for 'Interpolated height map.'
    let mut ihm = SquareArray::<f32>::new((PARCEL_MESH_STRIDE + 2) as usize, 0.);
    compute_interpolated_mesh(&mut ihm, shape_refs, &shapes_table);

    // `shm` stands for 'Smoothed height map`
    let mut shm = SquareArray::<f32>::new(PARCEL_MESH_STRIDE as usize, 0.);
    compute_smoothed_mesh(&mut shm, &ihm);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let mut position: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
    let mut indices: Vec<u32> = Vec::with_capacity((PARCEL_MESH_RESOLUTION.pow(2)) as usize);

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
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_aabb();
    Some(GroundMeshResult { mesh })
}

pub fn compute_interpolated_mesh(
    // `ihm` stands for 'Interpolated height map.'
    ihm: &mut SquareArray<f32>,
    shape_refs: [ShapeRef; ADJACENT_COUNT],
    shapes_table: &TerrainContoursTable,
) {
    let mut weights = SquareArray::<f32>::new((PARCEL_MESH_STRIDE + 2) as usize, 0.);

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
                    1 + x * PARCEL_MESH_RESOLUTION,
                    1 + z * PARCEL_MESH_RESOLUTION,
                    shape_ref.rotation as i32,
                );
            }
        }
    }

    for z in 0..=PARCEL_MESH_RESOLUTION + 2 {
        for x in 0..=PARCEL_MESH_RESOLUTION + 2 {
            let w = weights.get(x, z);
            if w > 0. {
                *ihm.get_mut_ref(x, z) /= w;
            }
        }
    }
}

pub fn compute_smoothed_mesh(shm: &mut SquareArray<f32>, ihm: &SquareArray<f32>) {
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
            *weight.get_mut_ref(x, z) += 1.0;
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
    h0 * (1. - fy) + h1 * fy
}
