//! A precinct's navigation input: what its mesh is built from, as plain data in the precinct's
//! own coordinates (its corner at the origin), so it can be built off the main thread or saved to
//! a file and built again without the game.

use std::path::Path;

use bevy::{ecs::system::SystemParam, prelude::*};
use panoply_core::{realm_physics::collision_groups, RealmPhysics, ToBevy, ToRapier};
use panoply_scenery::{
    precinct::Precinct, scenery_element::SceneryElement, ColliderDesc, ColliderShape, ColliderType,
    SceneryColliders, TerrainFxMap, PRECINCT_SIZE, PRECINCT_SIZE_F,
};
use panoply_terrain::{coarse_ground, Parcel, TerrainTypes, PARCEL_SIZE};
use rapier3d::prelude::{Collider, Group, RigidBodyHandle};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Parcels along each side of a precinct.
const PARCELS_PER_SIDE: i32 = PRECINCT_SIZE / PARCEL_SIZE;

/// Everything a precinct's navigation mesh is built from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavInput {
    /// The precinct's coordinates in its realm, in precincts.
    pub coords: IVec2,
    /// Scenery's colliders, doors and hints included, and the trunks of generated trees.
    pub colliders: Vec<NavCollider>,
    /// The ground of each parcel, and the precinct's floors.
    pub surfaces: Vec<NavSurface>,
    /// Whether each 1 m cell is a footpath (a cobbled terrain effect), row by row from
    /// the cell at (-1, -1): the precinct's cells and a ring of its neighbours' around them, so a
    /// generator sees where a path runs on across the line. [`PATH_GRID`] cells a side.
    #[serde(default)]
    pub paths: Vec<bool>,
}

/// Cells along a side of [`NavInput::paths`]: the precinct's and one either side.
pub const PATH_GRID: usize = PRECINCT_SIZE as usize + 2;

impl NavInput {
    /// Whether the 1 m cell with its corner at `x`, `z` in the precinct is a footpath; `false`
    /// beyond the ring around the precinct.
    pub fn path_at(&self, x: i32, z: i32) -> bool {
        let range = -1..=PRECINCT_SIZE;
        range.contains(&x)
            && range.contains(&z)
            && self
                .paths
                .get((x + 1) as usize + (z + 1) as usize * PATH_GRID)
                .copied()
                .unwrap_or(false)
    }
}

/// A collider descriptor where it stands: scenery's as its catalog gives it, so its type, margin
/// and walkability come along, and a tree trunk as a solid cylinder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavCollider {
    /// The element's position, in the precinct's coordinates.
    pub position: Vec3,
    /// The element's turn about y, in radians.
    pub facing: f32,
    pub collider: ColliderDesc,
}

/// A walkable surface as triangles, in the precinct's coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavSurface {
    pub kind: SurfaceKind,
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SurfaceKind {
    /// A parcel's terrain at one vertex per meter ([`coarse_ground`]), its holes left out.
    Ground,
    /// A precinct's floor region, a closed slab.
    Floor,
}

/// Why a precinct's input could not be captured.
#[derive(Debug, Error, PartialEq)]
pub enum CaptureError {
    #[error("precinct {0} is not loaded")]
    PrecinctNotLoaded(IVec2),
    #[error("parcel {0} is not loaded")]
    ParcelNotLoaded(IVec2),
    #[error("the realm has no physics")]
    NoPhysics,
}

impl NavInput {
    /// Reads an input written by [`Self::write`].
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        rmp_serde::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Writes the input as MessagePack, with field names, since `ColliderDesc` flattens its
    /// shape and leaves out defaults.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let bytes = rmp_serde::to_vec_named(self).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Orders colliders and surfaces by what they are, so capturing unchanged content writes the
    /// same bytes whatever order it loaded in.
    fn sort(&mut self) {
        let by_position = |a: Vec3, b: Vec3| {
            a.x.total_cmp(&b.x)
                .then(a.z.total_cmp(&b.z))
                .then(a.y.total_cmp(&b.y))
        };
        // Within an element, its descriptors keep their catalog order (the sort is stable).
        self.colliders
            .sort_by(|a, b| by_position(a.position, b.position));
        self.surfaces.sort_by(|a, b| {
            a.kind
                .cmp(&b.kind)
                .then_with(|| match (a.vertices.first(), b.vertices.first()) {
                    (Some(&a), Some(&b)) => by_position(a, b),
                    (a, b) => a.is_some().cmp(&b.is_some()),
                })
        });
    }
}

/// What [`NavInputCapture::capture`] reads.
#[derive(SystemParam)]
pub struct NavInputCapture<'w, 's> {
    precincts: Query<
        'w,
        's,
        (
            &'static Precinct,
            &'static Children,
            Option<&'static TerrainFxMap>,
        ),
    >,
    elements: Query<'w, 's, (&'static SceneryElement, &'static SceneryColliders)>,
    parcels: Query<'w, 's, &'static Parcel>,
    realms: Query<'w, 's, &'static RealmPhysics>,
}

impl NavInputCapture<'_, '_> {
    /// The input of the precinct at `coords` in `realm`: its elements' descriptors as they
    /// stand, and from physics its floors and its parcels' ground and trunks, copied rather than
    /// triangulated. Fails if the precinct or any of its parcels is not loaded.
    pub fn capture(&self, realm: Entity, coords: IVec2) -> Result<NavInput, CaptureError> {
        let (precinct, children, effects) = self
            .precincts
            .iter()
            .find(|(p, ..)| p.realm == realm && p.coords == coords)
            .ok_or(CaptureError::PrecinctNotLoaded(coords))?;
        let physics = self
            .realms
            .get(realm)
            .map_err(|_| CaptureError::NoPhysics)?;
        let origin = Vec3::new(coords.x as f32, 0., coords.y as f32) * PRECINCT_SIZE_F;
        let mut input = NavInput {
            coords,
            paths: footpaths(effects),
            ..default()
        };

        // Children that are not elements with colliders (tiers, floors) are skipped.
        for (element, colliders) in self.elements.iter_many(children).flatten() {
            input.colliders.extend(
                colliders
                    .0
                    .iter()
                    .filter(|desc| desc.kind != ColliderType::Marker)
                    .map(|desc| NavCollider {
                        position: element.position,
                        facing: element.facing,
                        collider: desc.clone(),
                    }),
            );
        }
        let floors = body_colliders(physics, precinct.physics, collision_groups::FLOOR);
        input
            .surfaces
            .extend(floors.filter_map(|c| surface(c, SurfaceKind::Floor, origin)));

        let first = coords * PARCELS_PER_SIDE;
        for z in first.y..first.y + PARCELS_PER_SIDE {
            for x in first.x..first.x + PARCELS_PER_SIDE {
                let parcel_coords = IVec2::new(x, z);
                let parcel = self
                    .parcels
                    .iter()
                    .find(|p| p.realm == realm && p.coords == parcel_coords)
                    .ok_or(CaptureError::ParcelNotLoaded(parcel_coords))?;
                let ground = body_colliders(physics, parcel.physics, collision_groups::TERRAIN);
                input
                    .surfaces
                    .extend(ground.filter_map(|c| surface(c, SurfaceKind::Ground, origin)));
                for trunks in body_colliders(physics, parcel.physics, collision_groups::FLORA) {
                    input.colliders.extend(trunk_colliders(trunks, origin));
                }
            }
        }
        input.sort();
        Ok(input)
    }
}

/// Each cell's footpath flag, from the precinct's terrain effects: a path is cobbled, as faery's
/// navigation took its road and stone channels (`getMovementEffectsMask`). Trodden ground
/// (`TerrainTypes::Path`, faery's dirt) is not a path.
fn footpaths(effects: Option<&TerrainFxMap>) -> Vec<bool> {
    let Some(effects) = effects else {
        return vec![false; PATH_GRID * PATH_GRID];
    };
    (-1..=PRECINCT_SIZE)
        .flat_map(|z| (-1..=PRECINCT_SIZE).map(move |x| (x, z)))
        .map(|(x, z)| {
            let fx = effects.effect_at(x, z);
            fx.effect_strength > 0. && fx.effect.contains(TerrainTypes::Cobbles)
        })
        .collect()
}

/// The colliders on `body` in `group`.
fn body_colliders(
    physics: &RealmPhysics,
    body: RigidBodyHandle,
    group: Group,
) -> impl Iterator<Item = &Collider> {
    let world = physics.world();
    world
        .bodies
        .get(body)
        .into_iter()
        .flat_map(|body| body.colliders())
        .filter_map(|&handle| world.colliders.get(handle))
        .filter(move |c| c.collision_groups().memberships.intersects(group))
}

/// A trimesh collider's triangles, moved into the precinct's coordinates: a floor's as they are,
/// the ground's at one vertex per meter.
fn surface(collider: &Collider, kind: SurfaceKind, origin: Vec3) -> Option<NavSurface> {
    let mesh = collider.shape().as_trimesh()?;
    let (vertices, indices) = match kind {
        SurfaceKind::Floor => (
            mesh.vertices().iter().map(|&v| v.to_bevy()).collect(),
            mesh.indices().to_vec(),
        ),
        SurfaceKind::Ground => {
            let coarse = coarse_ground(mesh.vertices(), mesh.indices());
            (coarse.vertices, coarse.indices)
        }
    };
    let pose = collider.position();
    Some(NavSurface {
        kind,
        vertices: vertices
            .into_iter()
            .map(|v: Vec3| pose.transform_point(v.to_rapier()).to_bevy() - origin)
            .collect(),
        indices,
    })
}

/// A trunk compound's capsules as solid cylinders standing where they stand: the same footprint
/// and height, and the rounded ends do not matter to navigation.
fn trunk_colliders(collider: &Collider, origin: Vec3) -> Vec<NavCollider> {
    let Some(compound) = collider.shape().as_compound() else {
        return Vec::new();
    };
    let pose = collider.position();
    compound
        .shapes()
        .iter()
        .filter_map(|(part, shape)| {
            let capsule = shape.as_capsule()?;
            let half_height = capsule.half_height() + capsule.radius;
            let center = (*pose * *part).translation.to_bevy() - origin;
            Some(NavCollider {
                position: center - Vec3::Y * half_height,
                facing: 0.,
                collider: ColliderDesc {
                    shape: ColliderShape::Cylinder {
                        radius: capsule.radius,
                        half_height,
                    },
                    ..default()
                },
            })
        })
        .collect()
}
