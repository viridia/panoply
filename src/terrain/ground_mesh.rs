use std::sync::{Arc, Mutex};

use crate::{
    terrain::{
        ground_material::ATTRIBUTE_TERRAIN_STYLE, TerrainOptions, TerrainTypes,
        PARCEL_MESH_STRIDE_U, PARCEL_TERRAIN_FX_SIZE, PARCEL_TERRAIN_FX_STRIDE,
    },
    world::Realm,
};

use super::{
    parcel::{Parcel, RebuildParcelGroundMesh, ShapeRef, ADJACENT_COUNT, CENTER_SHAPE},
    rotator::RotatingSquareArray,
    square::SquareArray,
    terrain_contours::{TerrainContoursHandle, TerrainContoursTable, TerrainContoursTableAsset},
    terrain_map::TerrainMap,
    TerrainFxVertexAttr, PARCEL_MESH_RESOLUTION, PARCEL_MESH_SCALE, PARCEL_MESH_SCALE_U,
    PARCEL_MESH_STRIDE, PARCEL_MESH_VERTEX_COUNT, PARCEL_SIZE, PARCEL_SIZE_F,
    PARCEL_TERRAIN_FX_AREA,
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

const TERRAIN_FX_FINE_SIZE: usize = PARCEL_TERRAIN_FX_SIZE * PARCEL_MESH_SCALE_U;

struct TerrainFxMap([TerrainFxVertexAttr; PARCEL_TERRAIN_FX_AREA]);

impl TerrainFxMap {
    #[inline(always)]
    pub fn get(&self, x: usize, z: usize) -> TerrainFxVertexAttr {
        self.0[x + z * PARCEL_TERRAIN_FX_STRIDE]
    }
}

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
        let Ok((realm, _map)) = realms_query.get(parcel.realm) else {
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
        let task = pool.spawn(async move {
            compute_ground_mesh(shape_refs, TerrainFxMap(terrain_fx), &contours)
        });
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
                                    .spawn((ground_mesh, realm.layer.clone()))
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
    terrain_fx: TerrainFxMap,
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
    let mut terrain_style: Vec<[u32; 2]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);

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
            normal.push(n.normalize().to_array());

            // let earth_fx = (if z > 40 && z < 60 && x > 10 && x < 30 {
            //     1.0
            // } else {
            //     0.0
            // } * 255.0) as u32;

            // let soil_fx = (if z > 10 && z < 30 && x > 40 && x < 60 {
            //     1.0
            // } else {
            //     0.0
            // } * 255.0) as u32;

            // let cobbles_fx = (if z > 10 && z < 30 && x > 10 && x < 30 {
            //     1.0
            // } else {
            //     0.0
            // } * 255.0) as u32;

            // let terrain_style_0: u32 = earth_fx | (soil_fx << 8) | (cobbles_fx << 24);
            // let terrain_style_1: u32 = 0;
            // terrain_style.push([terrain_style_0, terrain_style_1]);
        }
    }

    // Generate indices
    for z in 0..PARCEL_MESH_STRIDE - 1 {
        for x in 0..PARCEL_MESH_STRIDE - 1 {
            let tx = x * PARCEL_SIZE / PARCEL_MESH_RESOLUTION;
            let tz = z * PARCEL_SIZE / PARCEL_MESH_RESOLUTION;
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

    // Generate terrain styles
    let mut effect_strength: [[[f32; TERRAIN_FX_FINE_SIZE]; TERRAIN_FX_FINE_SIZE]; 3] =
        [[[0.0; TERRAIN_FX_FINE_SIZE]; TERRAIN_FX_FINE_SIZE]; 3];

    for z in 0..PARCEL_TERRAIN_FX_SIZE {
        for x in 0..PARCEL_TERRAIN_FX_SIZE {
            let fx = terrain_fx.get(x, z);
            for fx_type in 0..3 {
                if fx.effect.contains(TerrainTypes(1 << fx_type)) {
                    effect_strength[fx_type][x * 4 + 2][z * 4 + 2] = fx.effect_strength;
                }
            }

            // const elevation = toSigned(effect[ftAccessor.indexOf(x + 1, y + 1) * 8 + 4]);
            // maxStrength.fill(0);
            // for (let fx = 0; fx < 4; fx += 1) {
            //   const centerStrength = effect[ftAccessor.indexOf(x + 1, y + 1) * 8 + fx];
            //   if (centerStrength) {
            //     localStrength.fill(0);
            //     // const centerStrength = ft?.terrainEffectStrength ?? 1;
            //     for (let ry = -1; ry <= 1; ry += 1) {
            //       if (y + ry < -1 || y + ry > PLOT_LENGTH + 1) {
            //         continue;
            //       }
            //       if (ry !== 0 && !contY) {
            //         continue;
            //       }
            //       for (let rx = -1; rx <= 1; rx += 1) {
            //         if (x + rx < -1 || x + rx > PLOT_LENGTH + 1) {
            //           continue;
            //         }
            //         if (rx !== 0 && !contX) {
            //           continue;
            //         }
            //         let adjacentStrength =
            //           effect[ftAccessor.indexOf(x + rx + 1, y + ry + 1) * 8 + fx];

            //         if (rx !== 0 && ry && ry !== 0) {
            //           // Special case for corners - if both of the sides have strength, then
            //           // apply that to the corner.
            //           const adjacentX = effect[ftAccessor.indexOf(x + rx + 1, y + 1) * 8 + fx];
            //           const adjacentY = effect[ftAccessor.indexOf(x + 1, y + ry + 1) * 8 + fx];
            //           if (adjacentX && adjacentY) {
            //             adjacentStrength = Math.max(adjacentStrength, (adjacentX + adjacentY) * 0.5);
            //           }
            //         }

            //         // Strength is the average of this tile and adjacent tile strength.
            //         if (adjacentStrength !== 0) {
            //           localStrength[eIndex(2 + rx * 2, 2 + ry * 2)] =
            //             (centerStrength + adjacentStrength) * 0.5;
            //         }
            //       }
            //     }

            //     // Now fill in the rest of the local strength array
            //     for (let yl = 0; yl <= 2; yl += 2) {
            //       for (let xl = 0; xl <= 2; xl += 2) {
            //         const s00 = localStrength[eIndex(xl, yl)];
            //         const s10 = localStrength[eIndex(xl + 2, yl)];
            //         const s01 = localStrength[eIndex(xl, yl + 2)];
            //         const s11 = localStrength[eIndex(xl + 2, yl + 2)];
            //         localStrength[eIndex(xl + 1, yl)] = (s00 + s10) * 0.5;
            //         localStrength[eIndex(xl, yl + 1)] = (s00 + s01) * 0.5;
            //         localStrength[eIndex(xl + 1, yl + 2)] = (s01 + s11) * 0.5;
            //         localStrength[eIndex(xl + 2, yl + 1)] = (s10 + s11) * 0.5;
            //         localStrength[eIndex(xl + 1, yl + 1)] = (s00 + s01 + s10 + s11) * 0.25;
            //       }
            //     }

            //     // Now apply the effect to the terrain
            //     if (fx < 4) {
            //       for (let yl = 0; yl <= 4; yl += 1) {
            //         const ys = y * SUBTILE_RESOLUTION + yl;
            //         if (ys < 0 || ys > PLOT_SUBTILE_LENGTH) {
            //           continue;
            //         }
            //         for (let xl = 0; xl <= 4; xl += 1) {
            //           const xs = x * SUBTILE_RESOLUTION + xl;
            //           if (xs < 0 || xs > PLOT_SUBTILE_LENGTH) {
            //             continue;
            //           }
            //           const index = indexOf(x * SUBTILE_RESOLUTION + xl, y * SUBTILE_RESOLUTION + yl);
            //           const localIndex = eIndex(xl, yl);
            //           const effectiveStrength = localStrength[localIndex];
            //           maxStrength[localIndex] = Math.max(maxStrength[localIndex], effectiveStrength);
            //           if (effectiveStrength !== 0) {
            //             this.terrainStyle[index * 8 + fx] = Math.max(
            //               this.terrainStyle[index * 8 + fx],
            //               effectiveStrength
            //             );
            //           }
            //         }
            //       }
            //     }
            //   }
            // }

            // if (elevation) {
            //   for (let yl = 0; yl <= 4; yl += 1) {
            //     const ys = y * SUBTILE_RESOLUTION + yl;
            //     if (ys < 0 || ys > PLOT_SUBTILE_LENGTH) {
            //       continue;
            //     }
            //     for (let xl = 0; xl <= 4; xl += 1) {
            //       const xs = x * SUBTILE_RESOLUTION + xl;
            //       if (xs < 0 || xs > PLOT_SUBTILE_LENGTH) {
            //         continue;
            //       }
            //       const s = elevation * maxStrength[eIndex(xl, yl)] * (1 / (63 * 255));
            //       const index = indexOf(x * SUBTILE_RESOLUTION + xl, y * SUBTILE_RESOLUTION + yl);
            //       if (s > 0) {
            //         hOffset[index] = Math.max(hOffset[index], s);
            //       } else if (s < 0) {
            //         hOffset[index] = Math.min(hOffset[index], s);
            //       }
            //     }
            //   }
            // }

            // terrain_style.push([fx, 0]);
        }
    }

    // Propagate terrain effects in Z direction to mid-points between tiles.
    for z in 0..PARCEL_TERRAIN_FX_SIZE - 1 {
        for x in 0..PARCEL_TERRAIN_FX_SIZE {
            let fx0 = terrain_fx.get(x, z);
            let fx1 = terrain_fx.get(x, z + 1);
            let cont_z0 = fx0.options.contains(TerrainOptions::ContinuousY);
            let cont_z1 = fx0.options.contains(TerrainOptions::ContinuousY);
            if cont_z0 || cont_z1 {
                for fx_type in 0..3 {
                    if fx0.effect.contains(TerrainTypes(1 << fx_type))
                        && fx1.effect.contains(TerrainTypes(1 << fx_type))
                    {
                        effect_strength[fx_type][x * 4 + 2][z * 4 + 4] =
                            (fx0.effect_strength + fx1.effect_strength) * 0.5;
                    }
                }
            }
        }
    }

    // Propagate terrain effects in X direction to mid-points between tiles.
    for z in 0..PARCEL_TERRAIN_FX_SIZE {
        for x in 0..PARCEL_TERRAIN_FX_SIZE - 1 {
            let fx0 = terrain_fx.get(x, z);
            let fx1 = terrain_fx.get(x + 1, z);
            let cont_x0 = fx0.options.contains(TerrainOptions::ContinuousX);
            let cont_x1 = fx0.options.contains(TerrainOptions::ContinuousX);
            if cont_x0 || cont_x1 {
                for fx_type in 0..3 {
                    if fx0.effect.contains(TerrainTypes(1 << fx_type))
                        && fx1.effect.contains(TerrainTypes(1 << fx_type))
                    {
                        effect_strength[fx_type][x * 4 + 4][z * 4 + 2] =
                            (fx0.effect_strength + fx1.effect_strength) * 0.5;
                    }
                }
            }
        }
    }

    for z in 0..PARCEL_TERRAIN_FX_SIZE - 1 {
        for x in 0..PARCEL_TERRAIN_FX_SIZE - 1 {
            for fx_type in 0..3 {
                let lx = x * 4 + 4;
                let lz = z * 4 + 4;
                let s00 = effect_strength[fx_type][lx - 2][lz];
                let s01 = effect_strength[fx_type][lx + 2][lz];
                let s10 = effect_strength[fx_type][lx][lz - 2];
                let s11 = effect_strength[fx_type][lx][lz + 2];
                let count = (s00 > 0.0) as u32
                    + (s01 > 0.0) as u32
                    + (s10 > 0.0) as u32
                    + (s11 > 0.0) as u32;
                if count > 1 {
                    effect_strength[fx_type][lx][lz] = s00.max(s01).max(s10).max(s11);
                }
            }
        }
    }

    // Propagate terrain effects in Z direction to quad-points between tiles.
    for z in 0..PARCEL_TERRAIN_FX_SIZE - 1 {
        for x in 0..PARCEL_TERRAIN_FX_SIZE - 1 {
            let lx = x * 4;
            let lz = z * 4;
            for fx_type in 0..3 {
                effect_strength[fx_type][lx + 1][lz] =
                    (effect_strength[fx_type][lx][lz] + effect_strength[fx_type][lx + 2][lz]) * 0.5;
                effect_strength[fx_type][lx + 3][lz] = (effect_strength[fx_type][lx + 2][lz]
                    + effect_strength[fx_type][lx + 4][lz])
                    * 0.5;
                effect_strength[fx_type][lx + 1][lz + 2] = (effect_strength[fx_type][lx][lz + 2]
                    + effect_strength[fx_type][lx + 2][lz + 2])
                    * 0.5;
                effect_strength[fx_type][lx + 3][lz + 2] = (effect_strength[fx_type][lx + 2]
                    [lz + 2]
                    + effect_strength[fx_type][lx + 4][lz + 2])
                    * 0.5;

                effect_strength[fx_type][lx][lz + 1] =
                    (effect_strength[fx_type][lx][lz] + effect_strength[fx_type][lx][lz + 2]) * 0.5;
                effect_strength[fx_type][lx][lz + 3] = (effect_strength[fx_type][lx][lz + 2]
                    + effect_strength[fx_type][lx][lz + 4])
                    * 0.5;
                effect_strength[fx_type][lx + 2][lz + 1] = (effect_strength[fx_type][lx + 2][lz]
                    + effect_strength[fx_type][lx + 2][lz + 2])
                    * 0.5;
                effect_strength[fx_type][lx + 2][lz + 3] = (effect_strength[fx_type][lx + 2]
                    [lz + 2]
                    + effect_strength[fx_type][lx + 2][lz + 4])
                    * 0.5;

                effect_strength[fx_type][lx + 1][lz + 1] = (effect_strength[fx_type][lx][lz + 1]
                    + effect_strength[fx_type][lx + 1][lz]
                    + effect_strength[fx_type][lx + 2][lz + 1]
                    + effect_strength[fx_type][lx + 1][lz + 2])
                    * 0.25;
                effect_strength[fx_type][lx + 3][lz + 1] = (effect_strength[fx_type][lx + 2]
                    [lz + 1]
                    + effect_strength[fx_type][lx + 3][lz]
                    + effect_strength[fx_type][lx + 4][lz + 1]
                    + effect_strength[fx_type][lx + 3][lz + 2])
                    * 0.25;
                effect_strength[fx_type][lx + 1][lz + 3] = (effect_strength[fx_type][lx][lz + 3]
                    + effect_strength[fx_type][lx + 1][lz + 2]
                    + effect_strength[fx_type][lx + 2][lz + 3]
                    + effect_strength[fx_type][lx + 1][lz + 4])
                    * 0.25;
                effect_strength[fx_type][lx + 3][lz + 3] = (effect_strength[fx_type][lx + 2]
                    [lz + 3]
                    + effect_strength[fx_type][lx + 3][lz + 2]
                    + effect_strength[fx_type][lx + 4][lz + 3]
                    + effect_strength[fx_type][lx + 3][lz + 4])
                    * 0.25;
            }
        }
    }

    for z in 0..PARCEL_MESH_STRIDE_U {
        for x in 0..PARCEL_MESH_STRIDE_U {
            let cobbles_fx = (effect_strength[0][x + 4][z + 4] * 255.0) as u32;
            let soil_fx = (effect_strength[1][x + 4][z + 4] * 255.0) as u32;
            let earth_fx = (effect_strength[2][x + 4][z + 4] * 255.0) as u32;

            let terrain_style_0: u32 = pack_u32(cobbles_fx, soil_fx, earth_fx, 0);
            let terrain_style_1: u32 = 0;
            terrain_style.push([terrain_style_0, terrain_style_1]);
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
