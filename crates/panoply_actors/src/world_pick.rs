//! Picking in the 3D view during play: what the pointer is over, and the cursor it shows there.

use bevy::{
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        cursor::EntityCursor,
        events::PointerClick,
        hover::HoverMap,
        pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    },
    platform::collections::HashMap,
    prelude::*,
    window::{CursorOptions, CustomCursor, CustomCursorImage, PrimaryWindow},
};
use panoply_core::{
    realm_physics::{collision_groups, owner_shown},
    GameMode, PrimaryCamera, RealmPhysics, Viewpoint,
};
use panoply_exemplar::{hover_cursor, OwnedAspects, DEFAULT_INTERACTION_CURSOR};
use panoply_saga::SagaHost;

use crate::{
    actor_control::{PlayerActor, TROT_SPEED},
    actor_physics::WALKABLE_NORMAL_Y,
    RouteTo,
};

/// How far from the camera a pointer's ray can hit something, in meters.
const PICK_RANGE: f32 = 500.;

/// How near the point a ground click sends the player to is near enough, in meters (R92).
const CLICK_REACH: f32 = 0.2;

/// How far out from a clicked wall the ground under the click is looked for: past the band the
/// navigation mesh leaves along a wall for the agent's radius.
const WALL_STEP_OUT: f32 = 0.4;

/// Below every backend but the window's, which reports at negative infinity.
const BACKSTOP_ORDER: f32 = f32::MIN;

/// How long the pointer must rest before the cursor hides, in seconds.
const CURSOR_IDLE_SECONDS: f32 = 1.;

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
    commands
        .spawn((
            Name::new("WorldBackstop"),
            WorldBackstop,
            cursors.get(WORLD_CURSOR).clone(),
        ))
        .observe(route_to_click);
}

/// A click on the world where nothing offers an interaction sends the player there at a trot, by
/// a route (R92). A click on the sky, which hits nothing, is ignored.
fn route_to_click(
    mut click: On<PointerClick>,
    mode: Res<State<GameMode>>,
    player: Query<Entity, With<PlayerActor>>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary || *mode.get() != GameMode::Play {
        return;
    }
    click.propagate(false);
    let (Some(goal), Ok(player)) = (click.hit.position, player.single()) else {
        return;
    };
    commands
        .entity(player)
        .insert(RouteTo::new(goal, CLICK_REACH, TROT_SPEED));
}

/// Hides the cursor while it rests over the 3D view during play, as it would otherwise sit in the
/// middle of the scene while the player walks. It stays over the UI.
pub(crate) fn hide_idle_cursor(
    mut pointer: MessageReader<PointerInput>,
    time: Res<Time<Real>>,
    mode: Res<State<GameMode>>,
    hover: Res<HoverMap>,
    ui_nodes: Query<(), With<Node>>,
    mut options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut last_active: Local<f32>,
) {
    let active = pointer.read().any(|input| {
        input.pointer_id.is_mouse()
            && matches!(
                input.action,
                PointerAction::Move { .. } | PointerAction::Press(_)
            )
    });
    if active {
        *last_active = time.elapsed_secs();
    }
    let over_world = hover
        .get(&PointerId::Mouse)
        .is_some_and(|hovered| !hovered.keys().any(|entity| ui_nodes.contains(*entity)));
    let idle = time.elapsed_secs() - *last_active >= CURSOR_IDLE_SECONDS;
    let visible = !(*mode.get() == GameMode::Play && over_world && idle);
    if options.visible != visible {
        options.visible = visible;
    }
}

/// Where a click on the world at `hit` asks a route to: the hit itself, unless the surface there
/// is too steep to stand on, a wall; then the first surface below a point stepped out from the wall
/// toward the side it faces. So a click on a wall's outside walks to its foot, not to the floor
/// just behind it, which is nearer.
fn ground_point(
    physics: &RealmPhysics,
    hit: Vec3,
    normal: Vec3,
    shown: impl Fn(Entity) -> bool,
) -> Vec3 {
    if normal.y >= WALKABLE_NORMAL_Y {
        return hit;
    }
    let out = Vec3::new(normal.x, 0., normal.z).normalize_or_zero();
    let down = Ray3d::new(hit + out * WALL_STEP_OUT, Dir3::NEG_Y);
    let groups = collision_groups::BLOCKING.difference(collision_groups::CANOPY);
    physics
        .pick(down, PICK_RANGE, groups, shown)
        .map_or(hit, |(_, distance)| down.get_point(distance))
}

/// A picking backend. Casts each pointer's ray into the viewed realm's colliders, and reports the
/// first thing it hits if that offers an interaction, so that nothing is picked through a wall. Its
/// cursor is set here, on that entity alone, so it follows the entries offered at the moment. Actors
/// stop the ray as walls do. The canopy is passed through (D39), and so are the editor's markers
/// and whatever is hidden, such as cutaways (R15).
/// Every ray also reports the backstop, with the point the ray first hits, if any, which a click
/// on the backstop routes to.
pub(crate) fn pick_world(
    rays: Res<RayMap>,
    cameras: Query<&Camera, With<PrimaryCamera>>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&RealmPhysics>,
    interactions: Query<(&OwnedAspects, Option<&EntityCursor>)>,
    cursors: Res<WorldCursors>,
    backstop: Single<Entity, With<WorldBackstop>>,
    visibility: Query<&InheritedVisibility>,
    host: Res<SagaHost>,
    // Read for the interactions' `when`, so the hits are written through commands.
    ecs: &World,
    mut commands: Commands,
) {
    let host = host.get();
    let shown = owner_shown(&visibility);
    let physics = viewpoint.realm.and_then(|realm| realms.get(realm).ok());
    let groups = collision_groups::BLOCKING
        .union(collision_groups::ACTOR)
        .difference(collision_groups::CANOPY);
    for (&id, &ray) in rays.iter() {
        let Ok(camera) = cameras.get(id.camera) else {
            continue;
        };
        let Some(physics) = physics else {
            continue;
        };
        let picked = physics.pick_surface(ray, PICK_RANGE, groups, &shown);
        let point = picked.map(|(_, distance, normal)| {
            ground_point(physics, ray.get_point(distance), normal, &shown)
        });
        commands.write_message(PointerHits::new(
            id.pointer,
            vec![(*backstop, HitData::new(id.camera, 0., point, None))],
            BACKSTOP_ORDER,
        ));
        let Some((target, distance, _)) = picked else {
            continue;
        };
        let Ok((aspects, current)) = interactions.get(target) else {
            continue;
        };
        let Some(name) = hover_cursor(aspects, ecs, host, target) else {
            continue;
        };
        let cursor = cursors.get(&name);
        if current != Some(cursor) {
            commands.entity(target).try_insert(cursor.clone());
        }
        let hit = HitData::new(id.camera, distance, Some(ray.get_point(distance)), None);
        commands.write_message(PointerHits::new(
            id.pointer,
            vec![(target, hit)],
            camera.order as f32,
        ));
    }
}
