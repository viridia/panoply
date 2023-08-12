use bevy::{
    asset::LoadState,
    gltf::{Gltf, GltfExtras, GltfMesh},
    prelude::*,
    utils::HashMap,
};

/// Data for placing an individual model instance.
pub struct ModelPlacement {
    pub transform: Transform,
    pub visible: bool,
    // TODO: animation
}

/// Resource key for a GLTF model.
type ModelId = String;

/// Map of model resource names to instances, used in building the instance components.
pub type InstanceMap = HashMap<ModelId, Vec<ModelPlacement>>;

/// A model id and a list of instance placements. Typically this is built from the InstanceMap.
#[derive(Component)]
pub struct ModelPlacements {
    pub model: ModelId,
    pub placement_list: Vec<ModelPlacement>,
}

/// Marker component to let us know that the placement list has changed.
#[derive(Component)]
pub struct ModelPlacementChanged;

#[derive(Component)]
pub struct ModelInstances {
    pub handle: Handle<Gltf>,
    pub asset_path: String,
    pub needs_rebuild: bool,
}

#[derive(Bundle, Clone, Default)]
pub struct ModelInstanceBundle<M: Material> {
    pub mesh: Handle<GltfMesh>,
    pub material: Handle<M>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

pub fn create_mesh_instances(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &ModelPlacements,
        Option<&ModelPlacementChanged>,
        Option<&mut ModelInstances>,
    )>,
    // assets_gltf: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
    assets_gltf: Res<Assets<Gltf>>,
    mut assets_scene: ResMut<Assets<Scene>>,
    // assets_mesh: ResMut<Assets<Mesh>>,
    // assets_gltf_meshes: Res<Assets<GltfMesh>>,
) {
    for (entity, placements, pl_changed, model_instances) in query.iter_mut() {
        if pl_changed.is_some() {
            if let Some((fname, fragment)) = placements.model.split_once('#') {
                let handle: Handle<Gltf> = server.load(fname.clone());
                commands
                    .entity(entity)
                    .insert(ModelInstances {
                        handle,
                        asset_path: String::from(fragment),
                        needs_rebuild: true,
                    })
                    .remove::<ModelPlacementChanged>();
            }
        }

        if let Some(mut m_instances) = model_instances {
            let result = server.get_load_state(&m_instances.handle);
            if result == LoadState::Loaded {
                if m_instances.needs_rebuild {
                    m_instances.needs_rebuild = false;
                } else {
                    continue;
                }
                let mut children = Vec::<Entity>::new();
                let asset = assets_gltf.get(&m_instances.handle);
                if let Some(gltf) = asset {
                    if let Some(scene_handle) = gltf.named_scenes.get(&m_instances.asset_path) {
                        let scene = assets_scene.get_mut(&scene_handle).unwrap();
                        // println!("Scene found: [{}]", placements.model);

                        let mut _extras_query = scene.world.query::<(&Name, &GltfExtras)>();
                        // let mut entity_components: HashMap<Entity, Vec<Box<dyn Reflect>>> =
                        //     HashMap::new();
                        // for (name, extras) in extras_query.iter(&scene.world) {
                        //     println!("Name: {}, extras: {:?}", name, extras);
                        // }

                        let mut query = scene
                            .world
                            .query::<(&Handle<Mesh>, &Handle<StandardMaterial>)>();
                        // TODO: Replace material handle
                        // TODO: Cache mesh handle
                        // TODO: Instance mesh.
                        for (mesh, material) in query.iter(&scene.world) {
                            if placements.placement_list.len() < 3 {
                                for placement in placements.placement_list.iter() {
                                    children.push(
                                        commands
                                            .spawn(PbrBundle {
                                                mesh: mesh.clone(),
                                                material: material.clone(),
                                                transform: placement.transform,
                                                ..Default::default()
                                            })
                                            .id(),
                                    );
                                }
                            }
                            // if let Some(m) = assets_mesh.get(&mesh) {
                            //     println!(
                            //         "Name: {}, entity {:?}, parent: {:?}",
                            //         name, entity, parent
                            //     );
                            // }
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
