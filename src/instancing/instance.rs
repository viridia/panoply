use bevy::{asset::LoadState, gltf::Gltf, prelude::*, render::view::RenderLayers, utils::HashMap};
use serde::{Deserialize, Serialize};

/// Data for placing an individual model instance.
#[derive(Debug)]
pub struct ModelPlacement {
    pub transform: Transform,
    pub visible: bool,
    // TODO: animation
}

/// Resource key for a GLTF model.
type ModelId = String;

/// Map of model resource names to instances, used in building the instance components.
/// Each terrain parcel or scenery precinct will have one of these, which specifies how many
/// instances of each model are placed in the world, and where they are located.
pub type InstanceMap = HashMap<ModelId, Vec<ModelPlacement>>;

/// A model id and a list of model instance placements. Typically this is built from the InstanceMap.
/// This will be attached to an entity which will have additional components to hold the instanced
/// meshes.
#[derive(Component)]
pub struct ModelPlacements {
    pub model: ModelId,
    pub placement_list: Vec<ModelPlacement>,
    pub layer: RenderLayers,
}

/// Marker component to let us know that the placement list has changed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ModelPlacementChanged;

#[derive(Component)]
pub struct ModelInstances {
    /// Handle to the GLTF scene file.
    pub handle: Handle<Gltf>,
    /// Asset label of the model within the GLTF scene file.
    pub asset_label: String,
    pub needs_rebuild: bool,
}

// #[derive(Bundle, Clone, Default)]
// struct ModelInstanceBundle<M: Material> {
//     pub mesh: Handle<GltfMesh>,
//     pub material: Handle<M>,
//     pub transform: Transform,
//     pub global_transform: GlobalTransform,
//     pub visibility: Visibility,
//     pub computed_visibility: InheritedVisibility,
// }

/// Options contained in the [`GltfExtras`] field.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct MeshOptions {
    pub billboard: Option<bool>,
    pub outline: Option<f32>,
}

pub fn create_mesh_instances(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &ModelPlacements,
        Option<&ModelPlacementChanged>,
        Option<&mut ModelInstances>,
    )>,
    server: Res<AssetServer>,
    assets_gltf: Res<Assets<Gltf>>,
    mut assets_scene: ResMut<Assets<Scene>>,
) {
    for (entity, placements, pl_changed, model_instances) in query.iter_mut() {
        if pl_changed.is_some() {
            // Create an entity for each loaded model referenced by the ModelInstances.
            if let Some((fname, fragment)) = placements.model.split_once('#') {
                let handle: Handle<Gltf> = server.load(fname.to_owned());
                commands
                    .entity(entity)
                    .insert(ModelInstances {
                        handle,
                        asset_label: String::from(fragment),
                        needs_rebuild: true,
                    })
                    .remove::<ModelPlacementChanged>();
            }
        }

        if let Some(mut m_instances) = model_instances {
            let result = server.load_state(&m_instances.handle);
            if result == LoadState::Loaded {
                if m_instances.needs_rebuild {
                    m_instances.needs_rebuild = false;
                } else {
                    continue;
                }
                let mut children = Vec::<Entity>::new();
                let asset = assets_gltf.get(&m_instances.handle);
                if let Some(gltf) = asset {
                    // Lookup the GLTF Scene (which is the object we want to display) by name.
                    if let Some(scene_handle) = gltf.named_scenes.get(&m_instances.asset_label) {
                        let scene = assets_scene.get_mut(scene_handle).unwrap();
                        // println!("Model found: [{}]", placements.model);

                        // for placement in placements.placement_list.iter() {
                        //     children.push(
                        //         commands
                        //             .spawn(SceneBundle {
                        //                 scene: scene_handle.clone(),
                        //                 transform: placement.transform,
                        //                 ..Default::default()
                        //             })
                        //             .id(),
                        //     );
                        // }

                        // let mut mesh_options = MeshOptions::default();
                        // let mut extras_query = scene.world.query::<(&Name, &GltfExtras)>();
                        // let mut entity_components: HashMap<Entity, Vec<Box<dyn Reflect>>> =
                        //     HashMap::new();
                        // for (name, extras) in extras_query.iter(&scene.world) {
                        // mesh_options =
                        //     serde_json::from_str::<MeshOptions>(&extras.value).unwrap();
                        // println!("Name: {}, extras: {:?}", name, mesh_options);
                        // }

                        let mut query = scene
                            .world
                            .query::<(&Handle<Mesh>, &Handle<StandardMaterial>)>();
                        // TODO: Replace material handle
                        // TODO: Cache mesh handle
                        for (mesh, material) in query.iter(&scene.world) {
                            // Limit number of models for debugging.
                            if placements.placement_list.len() < usize::MAX {
                                for placement in placements.placement_list.iter() {
                                    children.push(
                                        commands
                                            .spawn((
                                                PbrBundle {
                                                    mesh: mesh.clone(),
                                                    material: material.clone(),
                                                    transform: placement.transform,
                                                    ..Default::default()
                                                },
                                                placements.layer,
                                            ))
                                            .id(),
                                    );
                                }
                            }
                        }
                    } else {
                        println!("Scene not found: [{}]", placements.model);
                        println!("Named scenes:");
                        for (s, _) in gltf.named_scenes.iter() {
                            println!(" * [{}]", s);
                        }
                    }
                } else {
                    panic!("Asset not found.");
                }
                commands.entity(entity).replace_children(&children);
            }
        }
    }
}
