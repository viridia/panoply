//! Hitch log, a development aid: logs each frame that takes much longer than usual, with what
//! the frames just before it spawned, to tell which kind of loading causes it.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    render::{render_resource::PipelineCache, Render, RenderApp, RenderSystems},
    world_serialization::WorldAssetRoot,
};
use panoply_core::RealmPhysics;
use panoply_terrain::Parcel;

use crate::scenery::{precinct::Precinct, scenery_element::SceneryElement};

/// A frame longer than this is logged: frames run about 8 ms at 120 fps, and a frame or two
/// past 14 ms already reads as a stutter.
const HITCH: Duration = Duration::from_millis(14);
/// Startup loads everything at once; hitches then are expected.
const SKIP_FRAMES: u32 = 240;
/// Frames of spawn counts kept, and printed with each hitch. Rendering runs a frame behind the
/// simulation, so the cost of what one frame spawns can land in the next.
const HISTORY: usize = 3;

pub struct HitchLogPlugin;

impl Plugin for HitchLogPlugin {
    fn build(&self, app: &mut App) {
        let pipelines = PipelineTotal::default();
        app.init_resource::<HitchLog>()
            .insert_resource(pipelines.clone())
            .add_systems(First, log_hitch)
            .add_systems(FixedLast, count_physics_step)
            .add_systems(Last, count_spawns);
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(pipelines)
                .add_systems(Render, count_pipelines.in_set(RenderSystems::Cleanup));
        }
    }
}

/// Number of pipelines in the render world's cache, shared with the main world. On macOS,
/// Bevy creates pipelines synchronously on the render thread, so new ones cost frame time.
#[derive(Resource, Clone, Default)]
struct PipelineTotal(Arc<AtomicUsize>);

/// Counts the pipelines, and logs each one created after startup, so that a hitch can be traced
/// to the material or mesh layout that first needed it.
fn count_pipelines(
    cache: Res<PipelineCache>,
    total: Res<PipelineTotal>,
    mut frames: Local<u32>,
    mut seen: Local<usize>,
) {
    use bevy::material::descriptor::PipelineDescriptor;

    *frames += 1;
    let count = cache.pipelines().count();
    if *frames > SKIP_FRAMES {
        for pipeline in cache.pipelines().skip(*seen) {
            let (label, defs) = match &pipeline.descriptor {
                PipelineDescriptor::RenderPipelineDescriptor(d) => (
                    d.label.as_deref(),
                    d.fragment.as_ref().map_or(&d.vertex.shader_defs, |f| &f.shader_defs),
                ),
                PipelineDescriptor::ComputePipelineDescriptor(d) => {
                    (d.label.as_deref(), &d.shader_defs)
                }
                PipelineDescriptor::MeshPipelineDescriptor(d) => (d.label.as_deref(), &Vec::new()),
            };
            info!(
                "Pipeline created: {} {:?}",
                label.unwrap_or("(unlabeled)"),
                defs
            );
        }
    }
    *seen = count;
    total.0.store(count, Ordering::Relaxed);
}

/// What one frame added.
#[derive(Default, Debug, Clone, Copy)]
struct Spawns {
    frame: u32,
    precincts: usize,
    parcels: usize,
    scenery: usize,
    /// glTF scenes requested (scenery models, flora, actors).
    scenes: usize,
    /// Mesh entities added, mostly from scenes instantiated.
    meshes: usize,
    /// Entities added that can be drawn (have render layers), a proxy for all entity spawns.
    drawable: usize,
    /// Physics steps run in this frame (fixed timestep, 64 Hz).
    steps: u32,
    /// Change in the number of physics colliders, all realms.
    colliders: isize,
    /// Parcels that went from hidden to shown (D20).
    shown: usize,
    /// Images added to `Assets<Image>` (loaded textures), uploaded to the GPU next frame.
    images: usize,
    /// Pipelines added to the render world's cache, as of the render world's last frame.
    pipelines: usize,
}

#[derive(Resource, Default)]
struct HitchLog {
    frame: u32,
    steps: u32,
    colliders: usize,
    pipelines: usize,
    history: VecDeque<Spawns>,
}

fn count_physics_step(mut log: ResMut<HitchLog>) {
    log.steps += 1;
}

fn count_spawns(
    mut log: ResMut<HitchLog>,
    precincts: Query<(), Added<Precinct>>,
    parcels: Query<(), Added<Parcel>>,
    scenery: Query<(), Added<SceneryElement>>,
    scenes: Query<(), Added<WorldAssetRoot>>,
    meshes: Query<(), Added<Mesh3d>>,
    drawable: Query<(), Added<RenderLayers>>,
    physics: Query<&RealmPhysics>,
    shown: Query<&Visibility, (Changed<Visibility>, With<Parcel>)>,
    mut images: MessageReader<AssetEvent<Image>>,
    pipeline_total: Res<PipelineTotal>,
) {
    log.frame += 1;
    let colliders = physics.iter().map(RealmPhysics::collider_count).sum::<usize>();
    let pipelines = pipeline_total.0.load(Ordering::Relaxed);
    let spawns = Spawns {
        frame: log.frame,
        precincts: precincts.iter().count(),
        parcels: parcels.iter().count(),
        scenery: scenery.iter().count(),
        scenes: scenes.iter().count(),
        meshes: meshes.iter().count(),
        drawable: drawable.iter().count(),
        steps: log.steps,
        colliders: colliders as isize - log.colliders as isize,
        shown: shown.iter().filter(|v| **v != Visibility::Hidden).count(),
        images: images
            .read()
            .filter(|ev| matches!(ev, AssetEvent::Added { .. }))
            .count(),
        pipelines: pipelines.saturating_sub(log.pipelines),
    };
    log.steps = 0;
    log.colliders = colliders;
    log.pipelines = pipelines;
    if log.history.len() == HISTORY {
        log.history.pop_front();
    }
    log.history.push_back(spawns);
}

/// Runs first in a frame, when `Time<Real>` holds the length of the frame before it.
fn log_hitch(log: Res<HitchLog>, time: Res<Time<Real>>) {
    let delta = time.delta();
    if log.frame < SKIP_FRAMES || delta < HITCH {
        return;
    }
    let spawns = log
        .history
        .iter()
        .map(|s| {
            format!(
                "[{}: {} precincts, {} parcels, {} shown, {} scenery, {} scenes, {} meshes, \
                 {} drawable, {} images, {} pipelines, {} steps, {:+} colliders]",
                s.frame,
                s.precincts,
                s.parcels,
                s.shown,
                s.scenery,
                s.scenes,
                s.meshes,
                s.drawable,
                s.images,
                s.pipelines,
                s.steps,
                s.colliders
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    info!(
        "Hitch: frame {} took {:.1} ms; spawned {spawns}",
        log.frame,
        delta.as_secs_f64() * 1000.
    );
}
