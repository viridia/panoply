use std::sync::{Arc, Mutex};

use crate::{
    terrain::{
        ground_material::ATTRIBUTE_TERRAIN_STYLE, TerrainOptions, TerrainTypes,
        PARCEL_TERRAIN_FX_SIZE,
    },
    world::Realm,
};

use super::{
    parcel::{Parcel, RebuildParcelGroundMesh, ShapeRef, ADJACENT_COUNT, CENTER_SHAPE},
    rotator::RotatingSquareArray,
    square::SquareArray,
    terrain_contours::{TerrainContoursHandle, TerrainContoursTable, TerrainContoursTableAsset},
    terrain_map::TerrainMap,
    ParcelTerrainFx, RebuildParcelTerrainFx, PARCEL_MESH_SCALE, PARCEL_MESH_SCALE_U,
    PARCEL_MESH_SIZE, PARCEL_MESH_STRIDE, PARCEL_MESH_VERTEX_COUNT, PARCEL_SIZE, PARCEL_SIZE_F,
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
use bevy_mod_picking::backends::raycast::RaycastPickable;
use futures_lite::future;

pub struct GroundMeshResult {
    mesh: Mesh,
}

#[derive(Component)]
pub struct ComputeGroundMeshTask(Task<Option<GroundMeshResult>>);

pub const HEIGHT_SCALE: f32 = 0.5;

pub const ATTRIBUTE_TERRAIN_FX: MeshVertexAttribute =
    MeshVertexAttribute::new("terrain_fx", 0x1000, VertexFormat::Uint8x4);

const TERRAIN_FX_FINE_SIZE: usize = PARCEL_TERRAIN_FX_SIZE * PARCEL_MESH_SCALE_U;

/// Spawns a task for each parcel to compute the ground mesh geometry.
#[allow(clippy::type_complexity)]
pub fn gen_ground_meshes(
    mut commands: Commands,
    mut q_parcels: Query<
        (Entity, &mut Parcel),
        (
            With<RebuildParcelGroundMesh>,
            Without<RebuildParcelTerrainFx>,
        ),
    >,
    q_realms: Query<(&Realm, &TerrainMap)>,
    server: Res<AssetServer>,
    ts_handle: Res<TerrainContoursHandle>,
    ts_assets: Res<Assets<TerrainContoursTableAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, parcel) in q_parcels.iter_mut() {
        let Ok((realm, _map)) = q_realms.get(parcel.realm) else {
            return;
        };

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
            pool.spawn(async move { compute_ground_mesh(shape_refs, &terrain_fx, &contours) });
        commands
            .entity(entity)
            .insert(ComputeGroundMeshTask(task))
            .insert(realm.layer.clone())
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
        if let Ok((realm, terrain_map)) = realms_query.get(parcel.realm) {
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
                            parcel.ground_entity = Some(
                                commands
                                    .spawn((
                                        ground_mesh,
                                        realm.layer.clone(),
                                        // TODO: Might want to pick on physics colliders instead.
                                        RaycastPickable,
                                    ))
                                    .set_parent(entity)
                                    .id(),
                            );
                        }
                    }
                }
                commands.entity(entity).remove::<ComputeGroundMeshTask>();
            }
        }
    }
}

#[allow(clippy::needless_range_loop)]
fn compute_ground_mesh(
    shape_refs: [ShapeRef; ADJACENT_COUNT],
    terrain_fx: &ParcelTerrainFx,
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
    let mut indices: Vec<u32> = Vec::with_capacity((PARCEL_MESH_SIZE.pow(2)) as usize);
    let mut terrain_style_f: Vec<[f32; 3]> = vec![[0., 0., 0.]; PARCEL_MESH_VERTEX_COUNT];
    let mut terrain_style: Vec<[u32; 2]> = vec![[0, 0]; PARCEL_MESH_VERTEX_COUNT];
    let mut terrain_elevation_offset: Vec<f32> = vec![0.; PARCEL_MESH_VERTEX_COUNT];

    for z in -1..PARCEL_SIZE + 1 {
        for x in -1..PARCEL_SIZE + 1 {
            let tfx = terrain_fx.get((x + 1) as usize, (z + 1) as usize);
            let cont_x = tfx.options.contains(TerrainOptions::ContinuousX);
            let cont_z = tfx.options.contains(TerrainOptions::ContinuousY);
            let x_span = if cont_x { -1..=1 } else { 0..=0 };
            let z_span = if cont_z { -1..=1 } else { 0..=0 };

            let elevation = tfx.elevation;
            let mut local_strength = [[[0.0; 5]; 5]; 3];
            // maxStrength.fill(0);
            for fx in 0..3 {
                let fx_mask = TerrainTypes(1 << fx);
                let has_effect = tfx.effect.contains(fx_mask);
                if has_effect {
                    // For each tile that has a terrain effect, determine whether any neighboring tiles
                    // have the same effect. Fill in the corners and sides of a 5x5 array.
                    let center_strength = tfx.effect_strength;
                    for rz in z_span.clone() {
                        if z + rz < -1 || z + rz > PARCEL_SIZE {
                            continue;
                        }
                        for rx in x_span.clone() {
                            if x + rx < -1 || x + rx > PARCEL_SIZE {
                                continue;
                            }
                            let adjacent =
                                terrain_fx.get((x + rx + 1) as usize, (z + rz + 1) as usize);
                            let has_adjacent = adjacent.effect.contains(fx_mask);
                            let mut adjacent_strength = if has_adjacent {
                                adjacent.effect_strength
                            } else {
                                0.0
                            };

                            if rx != 0 && rz != 0 {
                                // Special case for corners - if both of the sides have strength, then
                                // apply that to the corner.
                                let adjacent_x =
                                    terrain_fx.get((x + rx + 1) as usize, (z + 1) as usize);
                                let adjacent_z =
                                    terrain_fx.get((x + 1) as usize, (z + rz + 1) as usize);
                                if adjacent_x.effect.contains(fx_mask)
                                    && adjacent_z.effect.contains(fx_mask)
                                {
                                    adjacent_strength = adjacent_strength.max(
                                        (adjacent_x.effect_strength + adjacent_z.effect_strength)
                                            * 0.5,
                                    );
                                }
                            }

                            // Strength is the average of this tile and adjacent tile strength.
                            // In the center, adjacent_strength == center_strength.
                            if adjacent_strength != 0.0 {
                                local_strength[fx][(rx * 2 + 2) as usize][(rz * 2 + 2) as usize] =
                                    (center_strength + adjacent_strength) * 0.5;
                            }
                        }
                    }

                    // Now fill in the rest of the local strength array
                    for zl in [0, 2] {
                        for xl in [0, 2] {
                            let s00 = local_strength[fx][xl][zl];
                            let s10 = local_strength[fx][xl + 2][zl];
                            let s01 = local_strength[fx][xl][zl + 2];
                            let s11 = local_strength[fx][xl + 2][zl + 2];
                            local_strength[fx][xl + 1][zl] = (s00 + s10) * 0.5;
                            local_strength[fx][xl][zl + 1] = (s00 + s01) * 0.5;
                            local_strength[fx][xl + 1][zl + 2] = (s01 + s11) * 0.5;
                            local_strength[fx][xl + 2][zl + 1] = (s10 + s11) * 0.5;
                            local_strength[fx][xl + 1][zl + 1] = (s00 + s01 + s10 + s11) * 0.25;
                        }
                    }
                }

                if elevation != 0.0 {
                    for zl in 0..=4 {
                        let zs = z * 4 + zl;
                        if !(0..=PARCEL_MESH_SIZE).contains(&zs) {
                            continue;
                        }
                        for xl in 0..=4 {
                            let xs = x * 4 + xl;
                            if !(0..=PARCEL_MESH_SIZE).contains(&xs) {
                                continue;
                            }
                            let elevation =
                                elevation * local_strength[fx][xl as usize][zl as usize];
                            let index = (zs * PARCEL_MESH_STRIDE + xs) as usize;
                            let terrain_el = &mut terrain_elevation_offset[index];
                            if elevation > 0.0 {
                                *terrain_el = terrain_el.max(elevation);
                            } else if elevation < 0.0 {
                                *terrain_el = terrain_el.min(elevation);
                            }
                        }
                    }
                }

                // terrain_style.push([fx, 0]);
            }

            // Now apply the effect to the terrain
            for zl in 0..=4 {
                let zs = z * 4 + zl;
                if !(0..=PARCEL_MESH_SIZE).contains(&zs) {
                    continue;
                }
                for xl in 0..=4 {
                    let xs = x * 4 + xl;
                    if !(0..=PARCEL_MESH_SIZE).contains(&xs) {
                        continue;
                    }

                    let index = (zs * PARCEL_MESH_STRIDE + xs) as usize;

                    let fx_cobbles = local_strength[0][xl as usize][zl as usize];
                    let fx_soil = local_strength[1][xl as usize][zl as usize];
                    let fx_earth = local_strength[2][xl as usize][zl as usize];
                    let tsf = &mut terrain_style_f[index];
                    tsf[0] = tsf[0].max(fx_cobbles);
                    tsf[1] = tsf[1].max(fx_soil);
                    tsf[2] = tsf[2].max(fx_earth);
                }
            }
        }
    }

    for (index, tsf) in terrain_style_f.iter().enumerate() {
        let fx_cobbles = (tsf[0] * 255.0) as u32;
        let fx_soil = (tsf[1] * 255.0) as u32;
        let fx_earth = (tsf[2] * 255.0) as u32;
        let terrain_style_0: u32 = pack_u32(fx_cobbles, fx_soil, fx_earth, 0);
        let terrain_style_1: u32 = 0;
        terrain_style[index] = [terrain_style_0, terrain_style_1];
    }

    // Generate vertices
    let mut n = Vec3::new(0., 1., 0.);
    for z in 0..PARCEL_MESH_STRIDE {
        for x in 0..PARCEL_MESH_STRIDE {
            position.push([
                x as f32 * PARCEL_MESH_SCALE,
                shm.get(x, z) + terrain_elevation_offset[(z * PARCEL_MESH_STRIDE + x) as usize],
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
            normal.push(n.normalize().to_array());
        }
    }

    // Generate indices
    for z in 0..PARCEL_MESH_STRIDE - 1 {
        for x in 0..PARCEL_MESH_STRIDE - 1 {
            let tx = x * PARCEL_SIZE / PARCEL_MESH_SIZE;
            let tz = z * PARCEL_SIZE / PARCEL_MESH_SIZE;
            let fx = terrain_fx.get((tx + 1) as usize, (tz + 1) as usize);
            // Handle terrain holes.
            if fx.options.contains(TerrainOptions::Hole) {
                continue;
            }
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

    assert_eq!(position.len(), normal.len());
    assert_eq!(position.len(), terrain_style.len());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
    mesh.insert_attribute(ATTRIBUTE_TERRAIN_STYLE, terrain_style);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_aabb();
    Some(GroundMeshResult { mesh })
}

fn pack_u32(n0: u32, n1: u32, n2: u32, n3: u32) -> u32 {
    n0 | (n1 << 8) | (n2 << 16) | (n3 << 24)
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
                    1 + x * PARCEL_MESH_SIZE,
                    1 + z * PARCEL_MESH_SIZE,
                    shape_ref.rotation as i32,
                );
            }
        }
    }

    for z in 0..=PARCEL_MESH_SIZE + 2 {
        for x in 0..=PARCEL_MESH_SIZE + 2 {
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
    let x1 = (x_offset + PARCEL_MESH_SIZE + 1).min(dst.size() as i32);
    let z0 = z_offset.max(0);
    let z1 = (z_offset + PARCEL_MESH_SIZE + 1).min(dst.size() as i32);

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
    let cx = x.clamp(0., PARCEL_SIZE_F);
    let cz = z.clamp(0., PARCEL_SIZE_F);
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
