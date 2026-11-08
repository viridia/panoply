//! Picking in the 3D view during play: what the pointer is over, and the cursor it shows there.

use bevy::{
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        cursor::EntityCursor,
    },
    platform::collections::HashMap,
    prelude::*,
    window::{CustomCursor, CustomCursorImage},
};
use panoply_core::{realm_physics::collision_groups, PrimaryCamera, RealmPhysics, Viewpoint};
use panoply_exemplar::{Interactions, DEFAULT_INTERACTION_CURSOR};
use rapier3d::prelude::{Group, InteractionGroups, InteractionTestMode, QueryFilter, Ray};

/// How far from the camera a pointer's ray can hit something, in meters.
const PICK_RANGE: f32 = 500.;

/// Below every backend but the window's, which reports at negative infinity.
const BACKSTOP_ORDER: f32 = f32::MIN;

/// The cursor shown over the world when nothing there can be interacted with.
const WORLD_CURSOR: &str = "pointer";

/// Each cursor's name, its image, and the pixel of the image that points.
const CURSORS: [(&str, &str, (u16, u16)); 3] = [
    (WORLD_CURSOR, "cursors/pointer.png", (2, 2)),
    (DEFAULT_INTERACTION_CURSOR, "cursors/hand.png", (5, 2)),
    ("open-chest", "cursors/chest-open.png", (16, 12)),
];

/// The cursors that interactions can name.
#[derive(Resource, Default)]
pub(crate) struct WorldCursors(HashMap<&'static str, EntityCursor>);

impl WorldCursors {
    /// The cursor called `name`. An unknown name is an error, and shows the default.
    fn get(&self, name: &str) -> &EntityCursor {
        self.0.get(name).unwrap_or_else(|| {
            error_once!("Unknown interaction cursor '{name}'");
            &self.0[DEFAULT_INTERACTION_CURSOR]
        })
    }
}

/// Stands behind everything in the world during play: the pointer is over it when it is over
/// nothing else, so it carries the game's own cursor, which the editor's panels do not get. Bevy
/// reads no [`EntityCursor`] from a window, or the window could serve.
#[derive(Component)]
pub(crate) struct WorldBackstop;

pub(crate) fn spawn_world_backstop(
    server: Res<AssetServer>,
    mut cursors: ResMut<WorldCursors>,
    mut commands: Commands,
) {
    for (name, path, hotspot) in CURSORS {
        let image = CustomCursorImage {
            handle: server.load(path),
            hotspot,
            ..default()
        };
        cursors
            .0
            .insert(name, EntityCursor::Custom(CustomCursor::Image(image)));
    }
    commands.spawn((
        Name::new("WorldBackstop"),
        WorldBackstop,
        cursors.get(WORLD_CURSOR).clone(),
    ));
}

/// A picking backend. Casts each pointer's ray into the viewed realm's colliders, and reports the
/// first thing it hits if that offers an interaction, so that nothing is picked through a wall. Its
/// cursor is set here, on that entity alone, so it follows the entries offered at the moment. The
/// canopy is passed through (D39). Every ray also reports the backstop.
pub(crate) fn pick_world(
    rays: Res<RayMap>,
    cameras: Query<&Camera, With<PrimaryCamera>>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&RealmPhysics>,
    interactions: Query<(&Interactions, Option<&EntityCursor>)>,
    cursors: Res<WorldCursors>,
    backstop: Single<Entity, With<WorldBackstop>>,
    mut hits: MessageWriter<PointerHits>,
    mut commands: Commands,
) {
    let world = viewpoint
        .realm
        .and_then(|realm| realms.get(realm).ok())
        .map(RealmPhysics::world);
    let filter = QueryFilter::default().groups(InteractionGroups::new(
        Group::ALL,
        !collision_groups::CANOPY,
        InteractionTestMode::And,
    ));
    for (&id, ray) in rays.iter() {
        let Ok(camera) = cameras.get(id.camera) else {
            continue;
        };
        hits.write(PointerHits::new(
            id.pointer,
            vec![(*backstop, HitData::new(id.camera, 0., None, None))],
            BACKSTOP_ORDER,
        ));
        let ray = Ray::new(ray.origin, *ray.direction);
        let Some((handle, distance)) =
            world.and_then(|world| world.cast_ray(&ray, PICK_RANGE, true, filter))
        else {
            continue;
        };
        let Some(target) = world
            .and_then(|world| world.colliders.get(handle))
            .and_then(|collider| Entity::try_from_bits(collider.user_data as u64))
        else {
            continue;
        };
        let Ok((interactions, current)) = interactions.get(target) else {
            continue;
        };
        let Some(name) = interactions.hover_cursor() else {
            continue;
        };
        let cursor = cursors.get(name);
        if current != Some(cursor) {
            commands.entity(target).try_insert(cursor.clone());
        }
        let hit = HitData::new(id.camera, distance, Some(ray.point_at(distance)), None);
        hits.write(PointerHits::new(
            id.pointer,
            vec![(target, hit)],
            camera.order as f32,
        ));
    }
}
