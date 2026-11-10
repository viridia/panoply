use crate::{
    floor_region::{FloorRegion, RebuildFloorAspects, RebuildFloorMaterials, RebuildFloorMesh},
    floor_surface::FloorSurface,
    precinct::Precinct,
    FloorOutline, FLOOR_THICKNESS, PHYSICS_FLOOR_THICKNESS, TIER_OFFSET,
};
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use panoply_core::{realm_physics::collision_groups, OwnedColliders, RealmPhysics, ToRapier};
use panoply_render::FloorNoisyMaterial;
use rapier3d::prelude::{
    ColliderBuilder, Group, InteractionGroups, InteractionTestMode, SharedShape, TriMeshFlags,
};

pub struct FloorMeshResult {
    mesh: Mesh,
    outline: Option<Mesh>,
    collider: Option<SharedShape>,
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

/// Waits for a floor region's surface to load, then starts building its mesh and material. A
/// floor draws once it has both.
pub fn update_floor_aspects(
    mut commands: Commands,
    query: Query<(Entity, &FloorRegion), With<RebuildFloorAspects>>,
    server: Res<AssetServer>,
) {
    for (entity, floor_region) in query.iter() {
        let st = server.load_state(&floor_region.surface);
        if st.is_loaded() {
            commands
                .entity(entity)
                .remove::<RebuildFloorAspects>()
                .insert((RebuildFloorMaterials, RebuildFloorMesh));
        } else if st.is_failed() {
            commands.entity(entity).remove::<RebuildFloorAspects>();
        }
    }
}

/// Spawns a task for each floor region to compute its mesh geometry.
pub fn gen_floor_meshes(
    mut commands: Commands,
    query: Query<(Entity, &FloorRegion), With<RebuildFloorMesh>>,
    surfaces: Res<Assets<FloorSurface>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, floor_region) in query.iter() {
        let level = floor_region.level;
        let mut poly = floor_region.poly.clone();
        let holes = floor_region.holes.clone();
        if poly.last() == poly.first() {
            poly.pop();
        }
        let Some(surface) = surfaces.get(&floor_region.surface) else {
            continue;
        };
        let sides = surface.sides;
        let raise = surface.raise;
        let task = pool.spawn(async move {
            compute_floor_mesh(FloorMeshParams {
                level,
                poly,
                holes,
                has_texture: true,
                sides,
                raise,
            })
        });

        commands
            .entity(entity)
            .insert(ComputeFloorMeshTask(task))
            .remove::<RebuildFloorMesh>();
    }
}

/// Consumes the output of the compute task: the floor's mesh, its outline, and its collider on
/// the precinct's body (the floor region is a child of a tier, a child of the precinct).
pub(crate) fn insert_floor_meshes(
    mut commands: Commands,
    mut q_floors: Query<(Entity, &mut ComputeFloorMeshTask, &RenderLayers, &ChildOf)>,
    q_parents: Query<&ChildOf>,
    q_precincts: Query<&Precinct>,
    mut q_realms: Query<&mut RealmPhysics>,
    mut meshes: ResMut<Assets<Mesh>>,
    outline_material: Res<FloorOutline>,
) {
    for (entity, mut task, layers, tier) in q_floors.iter_mut() {
        let Some(task_result) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        let mut floor = commands.entity(entity);
        floor
            .remove::<ComputeFloorMeshTask>()
            .despawn_related::<Children>();
        // An outline that cannot be triangulated yields no mesh.
        let Some(task_result) = task_result else {
            floor.remove::<(Mesh3d, OwnedColliders)>();
            continue;
        };
        let precinct = q_parents
            .get(tier.parent())
            .and_then(|precinct| q_precincts.get(precinct.parent()));
        match (precinct, task_result.collider) {
            (Ok(precinct), Some(shape)) => {
                if let Ok(mut physics) = q_realms.get_mut(precinct.realm) {
                    let collider = ColliderBuilder::new(shape)
                        .collision_groups(InteractionGroups::new(
                            collision_groups::FLOOR,
                            Group::ALL,
                            InteractionTestMode::And,
                        ))
                        .user_data(entity.to_bits() as u128)
                        .build();
                    floor.insert(OwnedColliders {
                        realm: precinct.realm,
                        handles: vec![physics.insert_collider(precinct.physics, collider)],
                    });
                }
            }
            _ => {
                floor.remove::<OwnedColliders>();
            }
        }
        floor.insert(Mesh3d(meshes.add(task_result.mesh)));
        if let Some(outline_mesh) = task_result.outline {
            // The outline is a child sharing the floor's position (D14). Its mesh is its own,
            // with normals mitered at the corners, since the floor's sides have split normals.
            floor.with_child((
                Name::new("FloorOutline"),
                MeshMaterial3d(outline_material.0.clone()),
                Mesh3d(meshes.add(outline_mesh)),
                Visibility::Inherited,
                layers.clone(),
            ));
        }
    }
}

pub(crate) fn rebuild_floor_materials(
    mut commands: Commands,
    query: Query<(Entity, &FloorRegion), With<RebuildFloorMaterials>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    surfaces: Res<Assets<FloorSurface>>,
) {
    for (entity, region) in query.iter() {
        let Some(surface) = surfaces.get(&region.surface) else {
            continue;
        };
        let mut floor = commands.entity(entity);
        floor.remove::<RebuildFloorMaterials>();
        // An entity must not carry two materials (Notes, Bevy 0.20 issues), so remove the other.
        if let Some(ref noisy_material) = surface.material_noisy {
            floor
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(noisy_material.clone()));
        } else {
            let std_material = surface.material_std.clone().unwrap_or_else(|| {
                // Debug surface.
                std_materials.add(StandardMaterial {
                    base_color: Srgba::rgb(1.0, 0.0, 0.0).into(),
                    unlit: true,
                    ..Default::default()
                })
            });
            floor
                .remove::<MeshMaterial3d<FloorNoisyMaterial>>()
                .insert(MeshMaterial3d(std_material));
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
    let collider = compute_floor_collider(&params, &triangles);
    Some(FloorMeshResult {
        mesh,
        outline,
        collider,
    })
}

/// The floor as a closed trimesh, `PHYSICS_FLOOR_THICKNESS` thick under the drawn top surface
/// (which follows the surface's `raise`), holes included. One shape per region, so the internal
/// edges of its top are fixed up and leave no seams to catch on. Triangles and walls are
/// oriented here, outward, whatever the winding of the rings and of earcut's output.
fn compute_floor_collider(params: &FloorMeshParams, triangles: &[usize]) -> Option<SharedShape> {
    let (vertices, indices) = floor_collider_mesh(params, triangles);
    let vertices = vertices.into_iter().map(ToRapier::to_rapier).collect();
    SharedShape::trimesh_with_flags(vertices, indices, TriMeshFlags::FIX_INTERNAL_EDGES).ok()
}

fn floor_collider_mesh(
    params: &FloorMeshParams,
    triangles: &[usize],
) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let y_top = params.level as f32 + params.raise + TIER_OFFSET;
    let y_bottom = y_top - PHYSICS_FLOOR_THICKNESS;
    let rings: Vec<&[Vec2]> = std::iter::once(params.poly.as_slice())
        .chain(params.holes.iter().map(Vec::as_slice))
        .collect();
    let points: Vec<Vec2> = rings.iter().flat_map(|ring| ring.iter().copied()).collect();
    let count = points.len() as u32;
    let mut vertices: Vec<Vec3> = Vec::with_capacity(points.len() * 2);
    vertices.extend(points.iter().map(|p| Vec3::new(p.x, y_top, p.y)));
    vertices.extend(points.iter().map(|p| Vec3::new(p.x, y_bottom, p.y)));

    // In (x, z), a positive signed area turns away from +y, so the top wants negative areas.
    let area = |a: Vec2, b: Vec2, c: Vec2| (b - a).perp_dot(c - a);
    let mut indices: Vec<[u32; 3]> = Vec::with_capacity(triangles.len() * 2 / 3 + points.len() * 2);
    for &[i, j, k] in triangles.as_chunks::<3>().0 {
        let [a, b, c] = [i as u32, j as u32, k as u32];
        let down = area(points[i], points[j], points[k]) > 0.;
        let (top, bottom) = if down {
            ([a, c, b], [a + count, b + count, c + count])
        } else {
            ([a, b, c], [a + count, c + count, b + count])
        };
        indices.push(top);
        indices.push(bottom);
    }

    // A wall from `l` to `v` faces (dz, -dx), out of a ring of positive area; the outline should
    // face out of the floor, and a hole into the hole.
    let mut base = 0u32;
    for (ring_index, ring) in rings.iter().enumerate() {
        let n = ring.len() as u32;
        let ring_area: f32 = (0..ring.len())
            .map(|i| ring[i].perp_dot(ring[(i + 1) % ring.len()]))
            .sum();
        let flip = (ring_area > 0.) == (ring_index > 0);
        for i in 0..n {
            let (l, v) = (base + (i + n - 1) % n, base + i);
            let (lb, vb) = (l + count, v + count);
            if flip {
                indices.extend([[l, lb, v], [lb, vb, v]]);
            } else {
                indices.extend([[l, v, lb], [lb, v, vb]]);
            }
        }
        base += n;
    }
    (vertices, indices)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn square(min: f32, max: f32) -> Vec<Vec2> {
        vec![
            Vec2::new(min, min),
            Vec2::new(max, min),
            Vec2::new(max, max),
            Vec2::new(min, max),
        ]
    }

    /// Signed volume of the collider mesh: positive only if every face points out.
    fn collider_volume(poly: Vec<Vec2>, holes: Vec<Vec<Vec2>>) -> f32 {
        let mut flat: Vec<f64> = poly.iter().flat_map(|v| [v.x as f64, v.y as f64]).collect();
        let mut hole_starts = Vec::new();
        for hole in &holes {
            hole_starts.push(flat.len() / 2);
            flat.extend(hole.iter().flat_map(|v| [v.x as f64, v.y as f64]));
        }
        let triangles = earcutr::earcut(&flat, &hole_starts, 2).unwrap();
        let params = FloorMeshParams {
            level: 0,
            poly,
            holes,
            has_texture: false,
            sides: false,
            raise: 0.,
        };
        let (vertices, indices) = floor_collider_mesh(&params, &triangles);
        indices
            .iter()
            .map(|[a, b, c]| {
                let [a, b, c] = [a, b, c].map(|i| vertices[*i as usize]);
                a.dot(b.cross(c)) / 6.
            })
            .sum()
    }

    #[test]
    fn floor_collider_faces_out_for_either_winding() {
        let expected = (16. - 4.) * PHYSICS_FLOOR_THICKNESS;
        for (outer_reversed, hole_reversed) in [(false, false), (true, false), (false, true)] {
            let mut poly = square(0., 4.);
            let mut hole = square(1., 3.);
            if outer_reversed {
                poly.reverse();
            }
            if hole_reversed {
                hole.reverse();
            }
            let volume = collider_volume(poly, vec![hole]);
            assert!(
                (volume - expected).abs() < 1e-4,
                "winding ({outer_reversed}, {hole_reversed}): volume {volume}, expected {expected}"
            );
        }
    }
}
