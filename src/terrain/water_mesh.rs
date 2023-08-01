use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};

use bevy::{
    asset::LoadState,
    pbr::NotShadowCaster,
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};
use futures_lite::future;

use crate::world::Realm;

use super::{
    parcel::{Parcel, ParcelWaterChanged, ShapeRef},
    square::RotatingSquareArray,
    terrain_map::TerrainMap,
    terrain_shapes::{TerrainShapesAsset, TerrainShapesHandle, TerrainShapesTable},
    water_material::{WaterMaterialResource, ATTRIBUTE_DEPTH_MOTION},
    HEIGHT_SCALE, PARCEL_MESH_VERTEX_COUNT, PARCEL_SIZE_F, PARCEL_WATER_RESOLUTION,
    PARCEL_WATER_VERTEX_COUNT,
};

const WATER_HEIGHT: f32 = -0.4;

/// Spawns a task for each parcel to compute the water mesh geometry.
pub fn gen_water_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel), With<ParcelWaterChanged>>,
    realms_query: Query<(&Realm, &TerrainMap)>,
    server: Res<AssetServer>,
    ts_handle: Res<TerrainShapesHandle>,
    ts_assets: Res<Assets<TerrainShapesAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, parcel) in query.iter_mut() {
        let realm = realms_query.get(parcel.realm);
        if !realm.is_ok() {
            return;
        }

        if server.get_load_state(&ts_handle.0) != LoadState::Loaded {
            return;
        }

        let shapes = ts_assets
            .get(&ts_handle.0)
            .expect("asset shapes required")
            .0
            .clone();

        let shape_ref = parcel.shapes[4];

        let task = pool.spawn(async move { compute_water_mesh(shape_ref, &shapes) });
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
    // Reset the visibility bits for all parcels.
    for (entity, mut parcel, mut task) in query.iter_mut() {
        let realm = realms_query.get(parcel.realm);
        if realm.is_ok() {
            // Get layer bit from here. Or better from parcel.
            // let (_, terrain_map) = realm.unwrap();
            if let Some(mesh_result) = future::block_on(future::poll_once(&mut task.0)) {
                if let Some(mesh) = mesh_result {
                    let bundle = MaterialMeshBundle {
                        mesh: meshes.add(mesh),
                        material: material.handle.clone(),
                        transform: Transform::from_xyz(
                            parcel.coords.x as f32 * PARCEL_SIZE_F,
                            0.,
                            parcel.coords.y as f32 * PARCEL_SIZE_F,
                        ),
                        visibility: Visibility::Visible,
                        ..default()
                    };

                    // Add or replace water
                    match parcel.water_entity {
                        None => {
                            parcel.water_entity =
                                Some(commands.spawn((bundle, NotShadowCaster)).id());
                        }

                        Some(entity) => {
                            commands.entity(entity).insert(bundle);
                        }
                    }
                } else {
                    if let Some(entity) = parcel.water_entity {
                        commands.entity(entity).despawn_recursive();
                        parcel.water_entity = None;
                    }
                }
                commands.entity(entity).remove::<ComputeWaterMeshTask>();
            }
        }
    }
}

fn compute_water_mesh(
    shape_ref: ShapeRef,
    shapes: &Arc<Mutex<TerrainShapesTable>>,
) -> Option<Mesh> {
    let shapes_table = shapes.lock().unwrap();
    let terrain_shape = shapes_table.get(shape_ref.shape as usize);
    if !terrain_shape.has_water {
        return None;
    }

    let src_rot = RotatingSquareArray::new(
        terrain_shape.height.size(),
        shape_ref.rotation as i32,
        &terrain_shape.height.elts(),
    );

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let mut position: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_WATER_VERTEX_COUNT);
    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
    let mut depth_motion: Vec<[f32; 3]> = Vec::with_capacity(PARCEL_MESH_VERTEX_COUNT);
    let mut indices: Vec<u32> = Vec::with_capacity(PARCEL_WATER_VERTEX_COUNT);
    let index_map = HashMap::<UVec2, u32>::new();

    let parcel_center = PARCEL_SIZE_F * 0.5;
    let mut transform = Transform::IDENTITY;
    transform.rotate_around(
        Vec3::new(parcel_center, 0., parcel_center),
        Quat::from_rotation_y(-(shape_ref.rotation as f32) * PI * 0.5),
    );

    let n = Vec3::new(0., 1., 0.);

    let mut vertex_at = |x: usize, z: usize| {
        return match index_map.get(&UVec2::new(x as u32, z as u32)) {
            Some(&index) => index,
            None => {
                let depth = src_rot.get(x as i32, z as i32);
                let index = position.len() as u32;
                position.push([x as f32, WATER_HEIGHT, z as f32]);
                normal.push(n.to_array());
                depth_motion.push([depth as f32 * -HEIGHT_SCALE, 0., 0.]);
                index
            }
        };
    };

    for z in 0..PARCEL_WATER_RESOLUTION {
        for x in 0..PARCEL_WATER_RESOLUTION {
            // let da = src_rot.get(x as i32, z as i32);
            // let db = src_rot.get(x as i32 + 1, z as i32);
            // let dc = src_rot.get(x as i32 + 1, z as i32 + 1);
            // let dd = src_rot.get(x as i32, z as i32 + 1);
            // if da < 0 || db < 0 || dc < 0 || dd < 0 {
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
            // }
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
    mesh.insert_attribute(ATTRIBUTE_DEPTH_MOTION, depth_motion);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.compute_aabb();
    Some(mesh)
}
