use bevy::{asset::LoadState, gltf::Gltf, prelude::*, render::view::RenderLayers};

use panoply_exemplar::*;

use crate::models::PropagateRenderLayers;

use super::scenery_aspect::{ModelComponent, SceneryModels};

#[derive(Debug, Component, Default)]
pub struct SceneryElement {
    pub exemplar: Handle<Exemplar>,
    pub facing: f32,
    pub position: Vec3,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SceneryElementRebuildAspects;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SceneryElementRebuildModels;

#[derive(Debug, Component, Default)]
pub struct SceneryElementMesh {
    pub handle: Handle<Gltf>,
    pub label: String,
    pub placement: ModelComponent,
}

pub fn update_se_aspects(
    mut commands: Commands,
    mut q_elements: Query<(Entity, &SceneryElement), With<SceneryElementRebuildAspects>>,
    server: Res<AssetServer>,
) {
    for (entity, scenery_element) in q_elements.iter_mut() {
        let st = server.load_state(&scenery_element.exemplar);
        if st == LoadState::Loaded {
            commands
                .entity(entity)
                .add(UpdateAspects {
                    exemplar: scenery_element.exemplar.clone(),
                    finish: SceneryElementRebuildModels,
                })
                .remove::<SceneryElementRebuildAspects>();
        }
    }
}

pub fn spawn_se_models(
    mut commands: Commands,
    mut query: Query<(Entity, &SceneryModels, &RenderLayers), With<SceneryElementRebuildModels>>,
    // mut meshes: ResMut<Assets<Mesh>>,
    server: Res<AssetServer>,
) {
    for (entity, models, layers) in query.iter_mut() {
        commands.entity(entity).clear_children();
        for model in models.0.iter() {
            if let Some((fname, fragment)) = model.asset.split_once('#') {
                let handle: Handle<Gltf> = server.load(fname.to_owned());
                commands
                    .spawn((
                        SceneryElementMesh {
                            handle,
                            label: String::from(fragment),
                            placement: model.clone(),
                        },
                        layers.clone(),
                    ))
                    .set_parent(entity);
            }
        }
        commands
            .entity(entity)
            .remove::<SceneryElementRebuildModels>();
    }
}

pub fn spawn_se_model_instances(
    mut commands: Commands,
    mut query: Query<(Entity, &SceneryElementMesh), Without<Handle<Scene>>>,
    assets_gltf: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
) {
    for (entity, mesh) in query.iter_mut() {
        let result = server.load_state(&mesh.handle);
        if result == LoadState::Loaded {
            let asset = assets_gltf.get(&mesh.handle);
            if let Some(gltf) = asset {
                if let Some(scene_handle) = gltf.named_scenes.get(mesh.label.as_str()) {
                    let mut transform = Transform::from_translation(Vec3::new(0., 0., 0.));
                    component_transform(&mut transform, &mesh.placement);
                    commands.entity(entity).insert((
                        SceneBundle {
                            scene: scene_handle.clone(),
                            transform,
                            ..Default::default()
                        },
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

fn component_transform(transform: &mut Transform, placement: &ModelComponent) {
    // const nx = Math.round(position.x * 16);
    // const ny = Math.round(position.z * 16);
    // if (xRotationVariance) {
    //   xRotation = MathUtils.euclideanModulo(
    //     xRotation + (noise3(nx, ny, 11) - 0.5) * xRotationVariance,
    //     360
    //   );
    // }
    // if (yRotationVariance) {
    //   yRotation = MathUtils.euclideanModulo(
    //     yRotation + (noise3(nx, ny, 13) - 0.5) * yRotationVariance,
    //     360
    //   );
    // }
    // if (zRotationVariance) {
    //   zRotation = MathUtils.euclideanModulo(
    //     zRotation + (noise3(nx, ny, 14) - 0.5) * zRotationVariance,
    //     360
    //   );
    // }
    if let Some(offset) = placement.offset {
        transform.translation += offset;
    }
    if let Some(rot) = placement.x_rotation {
        transform.rotate_x(rot * std::f32::consts::PI / 180.0);
    }
    if let Some(rot) = placement.y_rotation {
        transform.rotate_y(rot * std::f32::consts::PI / 180.0);
    }
    if let Some(rot) = placement.z_rotation {
        transform.rotate_z(rot * std::f32::consts::PI / 180.0);
    }
    // if (scaleVariance) {
    //   scale += (noise3(nx, ny, 17) - 0.5) * scaleVariance;
    // }
    if let Some(scale) = placement.scale {
        transform.scale = Vec3::new(scale, scale, scale);
    }
}
