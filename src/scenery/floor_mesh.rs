use super::{
    floor_aspect::FloorSurface,
    floor_region::{FloorRegion, RebuildFloorAspects},
};
use crate::{
    scenery::{
        floor_region::{RebuildFloorMaterials, RebuildFloorMesh},
        FLOOR_THICKNESS, TIER_OFFSET,
    },
    schematic::UpdateAspects,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

pub struct FloorMeshResult {
    mesh: Mesh,
}

#[derive(Debug)]
pub struct FloorMeshParams {
    level: i32,
    poly: Vec<Vec2>,
    has_texture: bool,
    sides: bool,
    raise: f32,
}

#[derive(Component)]
pub(crate) struct ComputeFloorMeshTask(Task<Option<FloorMeshResult>>);

pub fn update_floor_aspects(
    mut commands: Commands,
    mut query: Query<(Entity, &mut FloorRegion), With<RebuildFloorAspects>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
) {
    if query.iter().count() == 0 {
        return;
    }

    // Placeholder material.
    let material = materials.add(StandardMaterial {
        base_color: Color::NONE,
        ..Default::default()
    });

    // Wait until schematic loaded before updating aspects
    for (entity, floor_region) in query.iter_mut() {
        let st = server.load_state(&floor_region.schematic);
        if st == LoadState::Loaded {
            commands
                .entity(entity)
                .insert((
                    RebuildFloorMesh,
                    MaterialMeshBundle {
                        material: material.clone(),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                ))
                .remove::<RebuildFloorAspects>()
                .add(UpdateAspects {
                    schematic: floor_region.schematic.clone(),
                    finish: RebuildFloorMaterials,
                });
        }
    }
}

/// Spawns a task for each parcel to compute the water mesh geometry.
pub fn gen_floor_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &FloorRegion), With<RebuildFloorMesh>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, floor_region) in query.iter_mut() {
        let level = floor_region.level;
        let poly = floor_region.poly.clone();
        let task = pool.spawn(async move {
            compute_floor_mesh(FloorMeshParams {
                level,
                poly,
                has_texture: true,
                sides: true,
                raise: 0.,
            })
        });

        commands
            .entity(entity)
            .insert(ComputeFloorMeshTask(task))
            .remove::<RebuildFloorMesh>();
    }
}

/// Consumes the output of the compute task and creates a mesh component for the floor geometry.
pub(crate) fn insert_floor_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut ComputeFloorMeshTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mut task) in query.iter_mut() {
        if let Some(Some(task_result)) = future::block_on(future::poll_once(&mut task.0)) {
            commands
                .entity(entity)
                .insert(meshes.add(task_result.mesh))
                .remove::<ComputeFloorMeshTask>();
        }
    }
}

pub(crate) fn rebuild_floor_materials(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&FloorSurface>), With<RebuildFloorMaterials>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, surf) in query.iter_mut() {
        if let Some(surf) = surf {
            let material = surf.material_handle.clone();
            commands
                .entity(entity)
                .insert((material, Visibility::Visible))
                .remove::<RebuildFloorMaterials>();
        } else {
            let material = materials.add(StandardMaterial {
                base_color: Color::rgb(0.0, 0.5, 0.5),
                // unlit: true,
                ..Default::default()
            });
            commands
                .entity(entity)
                .insert((material, Visibility::Visible))
                .remove::<RebuildFloorMaterials>();
        }
    }
}

fn compute_floor_mesh(params: FloorMeshParams) -> Option<FloorMeshResult> {
    let y_min = params.level as f32 - FLOOR_THICKNESS + TIER_OFFSET;
    let y_max = params.level as f32 + params.raise + TIER_OFFSET;
    let count = params.poly.len();
    let vertices: Vec<f64> = params
        .poly
        .iter()
        .flat_map(|v| vec![v.x as f64, v.y as f64])
        .collect();
    let Ok(triangles) = earcutr::earcut(&vertices, &[], 2) else {
        return None;
    };

    let top_vertex_count = count;
    let vertex_count = top_vertex_count * 2;

    let mut position: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut tex_coords: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);
    let mut indices: Vec<u32> = Vec::with_capacity(triangles.len());

    // Top surface:
    for pt in params.poly.iter() {
        position.push([pt.x, y_max, pt.y]);
        normal.push([0., 1., 0.]);

        if params.has_texture {
            tex_coords.push([pt.x, pt.y]);
        }
    }

    // Bottom surface - needed for shadows
    for pt in params.poly.iter() {
        position.push([pt.x, y_min, pt.y]);
        normal.push([0., -1., 0.]);

        if params.has_texture {
            tex_coords.push([pt.x, pt.y]);
        }
    }

    for i in triangles.iter() {
        indices.push(*i as u32);
    }
    indices.reverse();
    for i in triangles.iter() {
        indices.push((i + count) as u32);
    }

    // const bottomIndices = triangles.map(i => i + count);
    // const indices = [...topIndices, ...bottomIndices];

    // Sides
    if params.sides {
        let mut last: Vec2 = *params.poly.last().unwrap();
        let mut next_index = position.len() as u32;
        for v in params.poly {
            position.push([last.x, y_max, last.y]);
            position.push([last.x, y_min, last.y]);
            position.push([v.x, y_max, v.y]);
            position.push([v.x, y_min, v.y]);

            indices.extend([next_index, next_index + 2, next_index + 1]);
            indices.extend([next_index + 1, next_index + 2, next_index + 3]);
            next_index += 4;

            let mut normal2 = Vec2::new(v.y - last.y, last.x - v.x).normalize();
            normal.push([normal2.x, 0., normal2.y]);
            normal.push([normal2.x, 0., normal2.y]);
            normal.push([normal2.x, 0., normal2.y]);
            normal.push([normal2.x, 0., normal2.y]);

            normal2 *= 0.1;
            if params.has_texture {
                tex_coords.push([last.x, last.y]);
                tex_coords.push([last.x + normal2.x, last.y + normal2.y]);
                tex_coords.push([v.x, v.y]);
                tex_coords.push([v.x + normal2.x, v.y + normal2.y]);
            }

            last = v;
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_aabb();
    Some(FloorMeshResult { mesh })
}

// fn build_floor_outline_geometry(params: &FloorMeshParams) {
//     if !params.sides {
//       return;
//     }

//     let count = params.poly.length;
//     const triangles = earcut(polygon.flat());

//     const position: number[] = [];
//     const normal: number[] = [];

//     // Bottom surface
//     for (const [x, z] of polygon) {
//       position.push(x, this.height - FLOOR_THICKNESS, z);
//       normal.push(0, -1, 0);
//     }
//     const indices = [...triangles];

//     // Sides
//     let lastVtx2 = polygon.at(-2)!;
//     let lastVtx = polygon.at(-1)!;
//     const baseIndex = position.length / 3;
//     let polygonIndex = baseIndex;

//     vDir2.set(lastVtx[0] - lastVtx2[0], lastVtx[1] - lastVtx2[1]).normalize();
//     for (let i = 0; i < count; i++) {
//       const vtx = polygon[i];
//       const [lx, lz] = lastVtx;
//       const [x, z] = vtx;

//       vDir.set(x - lx, z - lz).normalize();

//       position.push(lx, this.height + raise, lz);
//       position.push(lx, this.height - FLOOR_THICKNESS, lz);

//       const dot = vDir3.addVectors(vDir, vDir2).normalize().dot(vDir);
//       const area = vDir2.x * vDir.y - vDir.x * vDir2.y;
//       if (dot < 0.8 && area > 0) {
//         // Acute angle requires beveling
//         position.push(lx, this.height, lz);
//         position.push(lx, this.height - FLOOR_THICKNESS, lz);

//         normal2.set(vDir2.y, -vDir2.x).normalize();
//         normal2.addScaledVector(vDir2, 0.2);
//         normal.push(normal2.x, 0, normal2.y);
//         normal.push(normal2.x, 0, normal2.y);

//         normal2.set(vDir.y, -vDir.x).normalize();
//         normal2.addScaledVector(vDir, -0.2);
//         normal.push(normal2.x, 0, normal2.y);
//         normal.push(normal2.x, 0, normal2.y);

//         const nextIndex = i < count - 1 ? polygonIndex + 4 : baseIndex;
//         indices.push(polygonIndex, polygonIndex + 2, polygonIndex + 1);
//         indices.push(polygonIndex + 1, polygonIndex + 2, polygonIndex + 3);

//         indices.push(polygonIndex + 2, nextIndex, polygonIndex + 3);
//         indices.push(polygonIndex + 3, nextIndex, nextIndex + 1);

//         // Bevel between side and bottom.
//         const bi = i > 0 ? i - 1 : count - 1;
//         indices.push(polygonIndex + 1, polygonIndex + 3, bi);
//         indices.push(nextIndex + 1, i, bi);
//         indices.push(polygonIndex + 3, nextIndex + 1, bi);

//         polygonIndex = nextIndex;
//       } else {
//         vCross.set(vDir.y + vDir2.y, -(vDir.x + vDir2.x)).normalize();

//         // Shallow angle gets mitered.
//         normal2.copy(vCross).multiplyScalar(1 / dot);
//         normal.push(normal2.x, 0, normal2.y);
//         normal.push(normal2.x, 0, normal2.y);

//         // Side quad
//         const nextIndex = i < count - 1 ? polygonIndex + 2 : baseIndex;
//         indices.push(polygonIndex, nextIndex, polygonIndex + 1);
//         indices.push(polygonIndex + 1, nextIndex, nextIndex + 1);

//         // Bevel between side and bottom.
//         const bi = i > 0 ? i - 1 : count - 1;
//         indices.push(nextIndex + 1, i, bi);
//         indices.push(polygonIndex + 1, nextIndex + 1, bi);

//         polygonIndex = nextIndex;
//       }

//       lastVtx2 = lastVtx;
//       lastVtx = vtx;
//       vDir2.copy(vDir);
//     }

//     const geometry = new BufferGeometry();
//     this.pool.add(geometry);

//     const positionBuffer = new Float32BufferAttribute(position, 3);
//     positionBuffer.needsUpdate = true;
//     geometry.setAttribute('position', positionBuffer);

//     const normalBuffer = new Float32BufferAttribute(normal, 3);
//     normalBuffer.needsUpdate = true;
//     geometry.setAttribute('normal', normalBuffer);

//     geometry.setIndex(indices);
//     geometry.computeBoundingBox();
//     geometry.computeBoundingSphere();

//     const mesh = new Mesh(geometry, outlineMaterial);
//     mesh.matrixAutoUpdate = false;
//     mesh.name = `FloorRegionOutline-${this.height}`;
//     mesh.receiveShadow = false;
//     mesh.castShadow = false;
//     mesh.updateMatrix();
//     this.group.add(mesh);
//     return mesh;
//   }
