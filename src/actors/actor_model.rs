use bevy::{prelude::*, render::view::RenderLayers};

use crate::models::PropagateRenderLayers;

use super::{actor_aspect::Skin, ActorRebuildModels};

#[derive(Debug, Component, Default)]
pub struct ActorModel {
    pub handle: Handle<Gltf>,
    pub label: String,
}

pub fn spawn_actor_models(
    mut commands: Commands,
    mut query: Query<(Entity, &Skin, &RenderLayers), With<ActorRebuildModels>>,
    _server: Res<AssetServer>,
) {
    for (entity, skin, _layers) in query.iter_mut() {
        // commands.entity(entity).clear_children();
        if let Some((fname, fragment)) = skin.0.split_once('#') {
            info!("Loading actor model: [{}#{}]", fname, fragment);
            // TODO: Re-enable this
            // let handle: Handle<Gltf> = server.load(fname.to_owned());
            // commands.entity(entity).insert((
            //     ActorModel {
            //         handle,
            //         label: String::from(fragment),
            //     },
            //     layers.clone(),
            // ));
        }
        commands.entity(entity).remove::<ActorRebuildModels>();
    }
}

pub fn spawn_actor_model_instances(
    mut commands: Commands,
    mut query: Query<(Entity, &ActorModel, &Transform), Without<SceneRoot>>,
    assets_gltf: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
) {
    for (entity, mesh, transform) in query.iter_mut() {
        let result = server.load_state(&mesh.handle);
        if result.is_loaded() {
            let asset = assets_gltf.get(&mesh.handle);
            if let Some(gltf) = asset {
                if let Some(scene_handle) = gltf.named_scenes.get(mesh.label.as_str()) {
                    info!("Spawning actor mesh: [{}]", mesh.label);
                    commands.entity(entity).insert((
                        SceneRoot(scene_handle.clone()),
                        *transform,
                        PropagateRenderLayers,
                    ));
                } else {
                    error!("Model not found: [{}]", mesh.label);
                    info!("Available scenes: [{:?}]", gltf.named_scenes.keys());
                    commands.entity(entity).despawn();
                    // panic!();
                }
            }
        }
    }
}
