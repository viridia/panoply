use std::sync::{Arc, Mutex};

use bevy::{
    asset::LoadState,
    pbr::NotShadowCaster,
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};
use bevy_mod_picking::backends::raycast::RaycastPickable;
use futures_lite::future;

use crate::world::Realm;

use super::{
    compute_interpolated_mesh, compute_smoothed_mesh,
    parcel::{Parcel, ParcelWaterChanged, ShapeRef, ADJACENT_COUNT},
    square::SquareArray,
    terrain_contours::{TerrainContoursHandle, TerrainContoursTable, TerrainContoursTableAsset},
    terrain_map::TerrainMap,
    water_material::{WaterMaterialResource, ATTRIBUTE_DEPTH_MOTION},
    HEIGHT_SCALE, PARCEL_MESH_STRIDE, PARCEL_MESH_VERTEX_COUNT, PARCEL_WATER_RESOLUTION,
    PARCEL_WATER_VERTEX_COUNT,
};

const WATER_HEIGHT: f32 = -0.4;

/// Spawns a task for each parcel to compute the water mesh geometry.
pub fn gen_water_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel), With<ParcelWaterChanged>>,
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

        let shapes = ts_assets
            .get(&ts_handle.0)
            .expect("asset shapes required")
            .0
            .clone();

        let shape_refs = parcel.contours;
        let task = pool.spawn(async move { compute_water_mesh(shape_refs, &shapes) });
        commands
            .entity(entity)
            .insert(ComputeWaterMeshTask(task))
            .remove::<ParcelWaterChanged>();
    }
}

#[derive(Component)]
pub struct ComputeWaterMeshTask(Task<Option<Mesh>>);

/// Consumes the output of the compute task and creates a mesh component for the water geometry.
pub fn insert_water_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel, &mut ComputeWaterMeshTask)>,
    realms_query: Query<(&Realm, &TerrainMap)>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<WaterMaterialResource>,
) {
    for (entity, mut parcel, mut task) in query.iter_mut() {
        if let Ok((realm, _terrain)) = realms_query.get(parcel.realm) {
            if let Some(mesh_result) = future::block_on(future::poll_once(&mut task.0)) {
                if let Some(mesh) = mesh_result {
                    // Add or replace water
                    match parcel.water_entity {
                        None => {
                            parcel.water_entity = Some(
                                commands
                                    .spawn((
                                        MaterialMeshBundle {
                                            mesh: meshes.add(mesh),
                                            material: material.handle.clone(),
                                            ..default()
                                        },
                                        NotShadowCaster,
                                        // Pickable {
                                        //     should_block_lower: true,
                                        //     is_hoverable: true,
                                        // },
                                        RaycastPickable,
                                        realm.layer.clone(),
                                    ))
                                    .set_parent(entity)
                                    .id(),
                            );
                        }

                        Some(ent) => {
                            commands.entity(ent).insert(meshes.add(mesh));
                        }
                    }
                } else if let Some(ent) = parcel.water_entity {
                    commands.entity(ent).despawn_recursive();
                    parcel.water_entity = None;
                }
                commands.entity(entity).remove::<ComputeWaterMeshTask>();
            }
        }
    }
}

fn compute_water_mesh(
    shape_refs: [ShapeRef; ADJACENT_COUNT],
    shapes: &Arc<Mutex<TerrainContoursTable>>,
) -> Option<Mesh> {
    let shapes_table = shapes.lock().unwrap();
    let terrain_shape = shapes_table.get(shape_refs[4].shape as usize);
    if !terrain_shape.has_water {
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
    let mut position: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_WATER_VERTEX_COUNT);
    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
    let mut depth_motion: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
    let mut indices: Vec<u32> = Vec::with_capacity(PARCEL_WATER_VERTEX_COUNT);
    let index_map = HashMap::<UVec2, u32>::new();

    let n = Vec3::new(0., 1., 0.);

    let mut vertex_at = |x: usize, z: usize| {
        return match index_map.get(&UVec2::new(x as u32, z as u32)) {
            Some(&index) => index,
            None => {
                let depth = shm.get(x as i32 * 2, z as i32 * 2);
                let index = position.len() as u32;
                position.push([x as f32 * 0.5, WATER_HEIGHT, z as f32 * 0.5]);
                normal.push(n.to_array());
                depth_motion.push([depth * -HEIGHT_SCALE, 0., 0.]);
                index
            }
        };
    };

    for z in 0..PARCEL_WATER_RESOLUTION {
        for x in 0..PARCEL_WATER_RESOLUTION {
            let da = shm.get(x as i32 * 2, z as i32 * 2);
            let db = shm.get(x as i32 * 2 + 1, z as i32 * 2);
            let dc = shm.get(x as i32 * 2 + 1, z as i32 * 2 + 1);
            let dd = shm.get(x as i32 * 2, z as i32 * 2 + 1);
            if da < 0. || db < 0. || dc < 0. || dd < 0. {
                let a = vertex_at(x, z);
                let b = vertex_at(x, z + 1);
                let c = vertex_at(x + 1, z + 1);
                let d = vertex_at(x + 1, z);
                indices.push(a);
                indices.push(b);
                indices.push(d);
                indices.push(b);
                indices.push(c);
                indices.push(d);
            }
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
    mesh.insert_attribute(ATTRIBUTE_DEPTH_MOTION, depth_motion);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_aabb();
    Some(mesh)
}
