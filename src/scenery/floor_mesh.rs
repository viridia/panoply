use super::{
    floor_aspect::{FloorGeometry, NoiseFloorSurface, StdFloorSurface},
    floor_region::{FloorRegion, RebuildFloorAspects},
    FloorOutline,
};
use crate::scenery::{
    floor_region::{RebuildFloorMaterials, RebuildFloorMesh},
    FLOOR_THICKNESS, TIER_OFFSET,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        view::RenderLayers,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use panoply_exemplar::UpdateAspects;

pub struct FloorMeshResult {
    mesh: Mesh,
    outline: Option<Mesh>,
}

#[derive(Debug)]
pub struct FloorMeshParams {
    level: i32,
    poly: Vec<Vec2>,
    holes: Vec<Vec<Vec2>>,
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

    // Wait until exemplar loaded before updating aspects
    for (entity, floor_region) in query.iter_mut() {
        let st = server.load_state(&floor_region.exemplar);
        if st == LoadState::Loaded {
            commands
                .entity(entity)
                .insert((MaterialMeshBundle {
                    material: material.clone(),
                    visibility: Visibility::Hidden,
                    ..default()
                },))
                .remove::<RebuildFloorAspects>()
                .add(UpdateAspects {
                    exemplar: floor_region.exemplar.clone(),
                    finish: (RebuildFloorMaterials, RebuildFloorMesh),
                });
        }
    }
}

/// Spawns a task for each parcel to compute the water mesh geometry.
pub fn gen_floor_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &FloorRegion, Option<&FloorGeometry>), With<RebuildFloorMesh>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, floor_region, floor_geometry) in query.iter_mut() {
        let level = floor_region.level;
        let mut poly = floor_region.poly.clone();
        let holes = floor_region.holes.clone();
        if poly.last() == poly.first() {
            poly.pop();
        }
        let geometry = match floor_geometry {
            Some(g) => *g,
            None => FloorGeometry::default(),
        };
        let task = pool.spawn(async move {
            compute_floor_mesh(FloorMeshParams {
                level,
                poly,
                holes,
                has_texture: true,
                sides: geometry.sides.unwrap_or(true),
                raise: geometry.raise.unwrap_or(0.),
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
    mut q_floors: Query<(Entity, &mut ComputeFloorMeshTask, &RenderLayers)>,
    mut meshes: ResMut<Assets<Mesh>>,
    outline_material: ResMut<FloorOutline>,
) {
    for (entity, mut task, layers) in q_floors.iter_mut() {
        if let Some(Some(task_result)) = future::block_on(future::poll_once(&mut task.0)) {
            let mesh = meshes.add(task_result.mesh);
            commands
                .entity(entity)
                .insert(mesh.clone())
                .remove::<ComputeFloorMeshTask>()
                .despawn_descendants();
            if let Some(outline_mesh) = task_result.outline {
                let outline_mesh = meshes.add(outline_mesh);
                let outline = commands
                    .spawn((
                        MaterialMeshBundle {
                            material: outline_material.0.clone(),
                            mesh: outline_mesh,
                            visibility: Visibility::Visible,
                            ..default()
                        },
                        layers.clone(),
                    ))
                    .id();
                commands.entity(entity).add_child(outline);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn rebuild_floor_materials(
    mut commands: Commands,
    mut query: Query<
        (Entity, Option<&StdFloorSurface>, Option<&NoiseFloorSurface>),
        With<RebuildFloorMaterials>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, surf, nsurf) in query.iter_mut() {
        if let Some(surf) = surf {
            // Standard material surface.
            // println!("Attaching material: {:?}", surf.material.path());
            commands
                .entity(entity)
                .insert((surf.material.clone(), Visibility::Visible))
                .remove::<RebuildFloorMaterials>();
        } else if let Some(proc_surface) = nsurf {
            // Procedural textured surface.
            let material = proc_surface.material.clone();
            commands
                .entity(entity)
                .remove::<Handle<StandardMaterial>>()
                .insert((material, Visibility::Visible))
                .remove::<RebuildFloorMaterials>();
        } else {
            // Debug surface.
            commands
                .entity(entity)
                .insert((
                    materials.add(StandardMaterial {
                        base_color: Srgba::rgb(1.0, 0.0, 0.0).into(),
                        unlit: true,
                        ..Default::default()
                    }),
                    Visibility::Visible,
                ))
                .remove::<RebuildFloorMaterials>();
        }
    }
}

fn compute_floor_mesh(params: FloorMeshParams) -> Option<FloorMeshResult> {
    let mut vertices: Vec<f64> = Vec::with_capacity(
        params.poly.len() * 2 + params.holes.iter().map(|p| p.len()).sum::<usize>() * 2,
    );
    let mut holes: Vec<usize> = Vec::with_capacity(params.holes.len());
    // Add the main polygon to vertices
    vertices.extend(
        params
            .poly
            .iter()
            .flat_map(|v| vec![v.x as f64, v.y as f64]),
    );
    // Now add the holes
    for hole in params.holes.iter() {
        holes.push(vertices.len() / 2);
        vertices.extend(hole.iter().flat_map(|v| vec![v.x as f64, v.y as f64]));
    }
    let Ok(triangles) = earcutr::earcut(&vertices, &holes, 2) else {
        return None;
    };
    if triangles.len() < 2 {
        return None;
    }

    let mesh = compute_floor_geometry(&params, &triangles);
    let outline = if params.sides {
        Some(compute_outline_geometry(&params, &triangles))
    } else {
        None
    };
    Some(FloorMeshResult { mesh, outline })
}

fn compute_floor_geometry(params: &FloorMeshParams, triangles: &[usize]) -> Mesh {
    let y_min = params.level as f32 - FLOOR_THICKNESS + TIER_OFFSET;
    let y_max = params.level as f32 + params.raise + TIER_OFFSET;
    let count = params.poly.len() + params.holes.iter().map(|p| p.len()).sum::<usize>();
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
    for hole in params.holes.iter() {
        for v in hole.iter() {
            position.push([v.x, y_max, v.y]);
            normal.push([0., 1., 0.]);

            if params.has_texture {
                tex_coords.push([v.x, v.y]);
            }
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

    for hole in params.holes.iter() {
        for v in hole.iter() {
            position.push([v.x, y_min, v.y]);
            normal.push([0., -1., 0.]);

            if params.has_texture {
                tex_coords.push([v.x, v.y]);
            }
        }
    }

    for i in triangles.iter() {
        indices.push(*i as u32);
    }
    indices.reverse();
    for i in triangles.iter() {
        indices.push((i + count) as u32);
    }

    // Sides
    if params.sides {
        let mut last: Vec2 = *params.poly.last().unwrap();
        let mut next_index = position.len() as u32;
        for v in params.poly.iter() {
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

            last = *v;
        }

        for hole in params.holes.iter() {
            let mut last: Vec2 = *hole.last().unwrap();
            for v in hole {
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

                last = *v;
            }
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
    mesh
}

fn compute_outline_geometry(params: &FloorMeshParams, triangles: &[usize]) -> Mesh {
    let y_min = params.level as f32 - FLOOR_THICKNESS + TIER_OFFSET;
    let y_max = params.level as f32 + params.raise + TIER_OFFSET;
    let count = params.poly.len();

    let vertex_count = count * 3;

    let mut position: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normal: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut indices: Vec<u32> = Vec::with_capacity(triangles.len() * 3);

    // Bottom surface
    for pt in params.poly.iter() {
        position.push([pt.x, y_min, pt.y]);
        normal.push([0., -1., 0.]);
    }
    for i in triangles.iter() {
        indices.push(*i as u32);
    }
    for hole in params.holes.iter() {
        for v in hole.iter() {
            position.push([v.x, y_min, v.y]);
            normal.push([0., -1., 0.]);
        }
    }

    // Sides and corners
    compute_outline_sides(
        &params.poly,
        0,
        y_max,
        y_min,
        &mut position,
        &mut normal,
        &mut indices,
    );

    let mut poly_base = count;
    for hole in params.holes.iter() {
        compute_outline_sides(
            hole,
            poly_base as u32,
            y_max,
            y_min,
            &mut position,
            &mut normal,
            &mut indices,
        );
        poly_base += hole.len();
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_aabb();
    mesh
}

fn compute_outline_sides(
    path: &[Vec2],
    path_base: u32,
    y_max: f32,
    y_min: f32,
    position: &mut Vec<[f32; 3]>,
    normal: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
) {
    let count = path.len();
    let last_vtx2 = path[count - 2];
    let mut last_vtx = path[count - 1];
    let mut v_dir2 = (last_vtx - last_vtx2).normalize();
    let base_index = position.len() as u32;
    let mut polygon_index = base_index;
    for (i, vtx) in path.iter().enumerate() {
        let Vec2 { x: lx, y: lz } = last_vtx;
        let v_dir = (*vtx - last_vtx).normalize();

        position.push([lx, y_max, lz]);
        position.push([lx, y_min, lz]);

        let dot = (v_dir + v_dir2).normalize().dot(v_dir);
        let area = v_dir2.x * v_dir.y - v_dir.x * v_dir2.y;
        if dot < 0.8 && area > 0. {
            // Acute angle requires beveling
            position.push([lx, y_max, lz]);
            position.push([lx, y_min, lz]);

            let normal2 = Vec2::new(v_dir2.y, -v_dir2.x).normalize() + v_dir2 * 0.2;
            // normal2.addScaledVector(vDir2, 0.2);
            normal.push([normal2.x, 0., normal2.y]);
            normal.push([normal2.x, 0., normal2.y]);

            let normal2 = Vec2::new(v_dir.y, -v_dir.x).normalize() + v_dir * -0.2;
            // normal2.addScaledVector(vDir, -0.2);
            normal.push([normal2.x, 0., normal2.y]);
            normal.push([normal2.x, 0., normal2.y]);

            let next_index: u32 = if i < count - 1 {
                polygon_index + 4
            } else {
                base_index
            };
            indices.extend([polygon_index, polygon_index + 2, polygon_index + 1]);
            indices.extend([polygon_index + 1, polygon_index + 2, polygon_index + 3]);

            indices.extend([polygon_index + 2, next_index, polygon_index + 3]);
            indices.extend([polygon_index + 3, next_index, next_index + 1]);

            // Bevel between side and bottom.
            let bi: u32 = if i > 0 { i - 1 } else { count - 1 } as u32;
            indices.extend([polygon_index + 1, polygon_index + 3, path_base + bi]);
            indices.extend([next_index + 1, i as u32, path_base + bi]);
            indices.extend([polygon_index + 3, next_index + 1, path_base + bi]);

            polygon_index = next_index;
        } else {
            let v_cross = Vec2::new(v_dir.y + v_dir2.y, -(v_dir.x + v_dir2.x)).normalize();

            // Shallow angle gets mitered.
            let normal2 = v_cross / dot;
            normal.push([normal2.x, 0., normal2.y]);
            normal.push([normal2.x, 0., normal2.y]);

            // Side quad
            let next_index: u32 = if i < count - 1 {
                polygon_index + 2
            } else {
                base_index
            };
            indices.extend([polygon_index, next_index, polygon_index + 1]);
            indices.extend([polygon_index + 1, next_index, next_index + 1]);

            // Bevel between side and bottom.
            let bi = if i > 0 { i - 1 } else { count - 1 } as u32;
            indices.extend([next_index + 1, path_base + i as u32, path_base + bi]);
            indices.extend([polygon_index + 1, next_index + 1, path_base + bi]);

            polygon_index = next_index;
        }

        last_vtx = *vtx;
        v_dir2 = v_dir;
    }
}
