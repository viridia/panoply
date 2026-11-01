//! Pipeline warm-up. Bevy creates a render pipeline the first time a combination of material
//! features and mesh layout is drawn, and on macOS it does so synchronously on the render
//! thread, about 35 ms each. Without warm-up that happens mid-scroll, the first time a kind of
//! tree or building comes into view. So every model catalog is loaded up front, and when one
//! loads, each of its scenes is drawn once, tiny and below the ground at the view center, then
//! despawned.

use bevy::{
    app::Propagate,
    asset::LoadedFolder,
    ecs::system::SystemParam,
    gltf::Gltf,
    prelude::*,
    world_serialization::{
        WorldAsset, WorldAssetRoot, WorldInstance, WorldInstanceReady, WorldInstanceSpawner,
    },
};
use panoply_core::{Realm, Viewpoint};

/// Model folders loaded at startup so that their pipelines are warmed before play.
const FOLDERS: [&str; 3] = ["terrain/models", "terrain/models-new", "scenery/models"];
/// Scenes spawned per frame, to keep warm-up itself from stalling a frame.
const SCENES_PER_FRAME: usize = 32;
/// Frames a warm-up instance stays after spawning: rendering runs a frame behind, and both the
/// main pass and the shadow pass must see it.
const FRAMES_SHOWN: u32 = 4;
/// Below the view center, so the ground hides it; small, so nothing pokes through.
const DEPTH: f32 = 2.;
const SCALE: f32 = 0.01;

pub struct WarmupPlugin;

impl Plugin for WarmupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WarmupQueue>()
            .add_systems(Startup, load_model_folders)
            .add_systems(Update, (queue_loaded_models, spawn_warmups, expire_warmups))
            .add_observer(start_warmup_countdown);
    }
}

/// Keeps the model folders loaded.
#[derive(Resource)]
pub struct ModelFolders(Vec<Handle<LoadedFolder>>);

/// Scenes waiting to be drawn once.
#[derive(Resource, Default)]
pub struct WarmupQueue(Vec<Handle<WorldAsset>>);

/// A scene instance drawn only to warm up its pipelines.
#[derive(Component)]
struct ModelWarmup {
    /// Frames left once the scene has spawned; `None` until then.
    frames_left: Option<u32>,
}

fn load_model_folders(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(ModelFolders(
        FOLDERS
            .iter()
            .map(|path| server.load_folder(*path))
            .collect(),
    ));
}

/// Queues every scene of each glTF file as it loads, whether from the folders above or later.
fn queue_loaded_models(
    mut events: MessageReader<AssetEvent<Gltf>>,
    gltfs: Res<Assets<Gltf>>,
    mut queue: ResMut<WarmupQueue>,
) {
    for event in events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if let Some(gltf) = gltfs.get(*id) {
                queue.0.extend(gltf.scenes.iter().cloned());
            }
        }
    }
}

fn spawn_warmups(
    mut commands: Commands,
    mut queue: ResMut<WarmupQueue>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
) {
    // Warm-up needs the camera rendering a realm, so the scenes land in its view.
    let Some(realm) = viewpoint.realm.and_then(|realm| realms.get(realm).ok()) else {
        return;
    };
    let count = queue.0.len().min(SCENES_PER_FRAME);
    let transform = Transform::from_translation(viewpoint.position - Vec3::Y * DEPTH)
        .with_scale(Vec3::splat(SCALE));
    for scene in queue.0.drain(..count) {
        commands.spawn((
            Name::new("ModelWarmup"),
            ModelWarmup { frames_left: None },
            WorldAssetRoot(scene),
            transform,
            Propagate(realm.layer.clone()),
        ));
    }
}

fn start_warmup_countdown(ready: On<WorldInstanceReady>, mut warmups: Query<&mut ModelWarmup>) {
    if let Ok(mut warmup) = warmups.get_mut(ready.entity) {
        warmup.frames_left = Some(FRAMES_SHOWN);
    }
}

/// Model work still to do: model folders loading, scenes waiting for warm-up or being drawn for
/// it, and scene instances (flora, scenery, warm-ups, anything with a `WorldAssetRoot`) that
/// Bevy has not yet spawned. For readiness checks, such as the editor's `ViewReady`.
#[derive(SystemParam)]
pub struct ModelsPending<'w, 's> {
    server: Res<'w, AssetServer>,
    folders: Option<Res<'w, ModelFolders>>,
    queue: Res<'w, WarmupQueue>,
    warmups: Query<'w, 's, (), With<ModelWarmup>>,
    spawner: Res<'w, WorldInstanceSpawner>,
    scenes: Query<'w, 's, Option<&'static WorldInstance>, With<WorldAssetRoot>>,
}

impl ModelsPending<'_, '_> {
    /// Names the first kind of work still pending, or `None` when there is none.
    pub fn waiting_on(&self) -> Option<&'static str> {
        let folders_loaded = self.folders.as_ref().is_some_and(|folders| {
            folders
                .0
                .iter()
                .all(|folder| self.server.is_loaded_with_dependencies(folder))
        });
        if !folders_loaded {
            Some("model folders")
        } else if !self.queue.0.is_empty() || !self.warmups.is_empty() {
            Some("warm-up")
        } else if self
            .scenes
            .iter()
            .any(|instance| !instance.is_some_and(|id| self.spawner.instance_is_ready(**id)))
        {
            Some("scene instances")
        } else {
            None
        }
    }
}

fn expire_warmups(mut commands: Commands, mut warmups: Query<(Entity, &mut ModelWarmup)>) {
    for (entity, mut warmup) in &mut warmups {
        match warmup.frames_left {
            Some(0) => commands.entity(entity).despawn(),
            Some(ref mut frames) => *frames -= 1,
            None => {}
        }
    }
}
