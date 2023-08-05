use bevy::{
    asset::LoadState,
    gltf::{Gltf, GltfMesh},
    prelude::*,
    utils::HashMap,
};

/// Data for placing an individual instance.
pub struct InstancePlacement {
    pub transform: Transform,
    pub visible: bool,
    // TODO: animation
}

/// Resource key for a GLTF model.
type ModelId = String;

/// Map of model resource names to instances, used in building the instance components.
type InstanceMap = HashMap<ModelId, Vec<InstancePlacement>>;

/// A model id and a list of instance placements. Typically this is built from the InstanceMap.
#[derive(Component)]
pub struct InstancePlacementList {
    model: ModelId,
    placement_list: Vec<InstancePlacement>,
}

/// Marker component to let us know that the placement list has changed.
/// TODO: Not sure we will need this.
#[derive(Component)]
pub struct InstancePlacementChanged;

#[derive(Component)]
pub struct ModelInstanceRequest {
    pub model: String,
    pub transform: Transform,
    pub visible: bool,
}

#[derive(Component)]
pub struct ModelInstanceMesh {
    pub handle: Handle<Gltf>,
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
        &ModelInstanceRequest,
        Changed<ModelInstanceRequest>,
        Option<&mut ModelInstanceMesh>,
    )>,
    // assets_gltf: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
    assets_gltf: Res<Assets<Gltf>>,
    assets_gltf_meshes: Res<Assets<GltfMesh>>,
) {
    for (entity, minst, model_changed, mhandle) in query.iter_mut() {
        if model_changed {
            let handle: Handle<Gltf> = server.load(minst.model.clone());
            commands.entity(entity).insert(ModelInstanceMesh {
                handle,
                needs_rebuild: true,
            });
        }

        if let Some(mut mesh) = mhandle {
            if mesh.needs_rebuild {
                mesh.needs_rebuild = false;
            } else {
                continue;
            }
            let result = server.get_load_state(&mesh.handle);
            let mut children = Vec::<Entity>::new();
            if result == LoadState::Loaded {
                let asset = assets_gltf.get(&mesh.handle);
                if let Some(gltf) = asset {
                    // println!("Primitives: {}", gltf.primitives.len());
                    for mesh_handle in gltf.meshes.iter() {
                        if let Some(mesh) = assets_gltf_meshes.get(&mesh_handle) {
                            children.push(
                                commands
                                    .spawn(PbrBundle {
                                        mesh: mesh.primitives[0].mesh.clone(),
                                        // (unwrap: material is optional, we assume this primitive has one)
                                        material: mesh.primitives[0].material.clone().unwrap(),
                                        transform: minst.transform,
                                        ..Default::default()
                                    })
                                    .id(),
                            );
                        }
                    }
                    commands.entity(entity).replace_children(&children);
                } else {
                    println!("Asset not found: [{}]", minst.model);
                    // if let Some((fname, fragment)) = minst.model.split_once('#') {
                    //     println!("Named scenes:");
                    //     for (s, _) in root.named_scenes.iter() {
                    //         println!(" * [{}]", s);
                    //     }
                    //     println!("Named meshes:");
                    //     for (s, _) in root.named_meshes.iter() {
                    //         println!(" * [{}]", s);
                    //     }
                    //     println!("Unnamed meshes: {}", root.meshes.len());
                    //     println!("Named nodes:");
                    //     for (s, _) in root.named_nodes.iter() {
                    //         println!(" * [{}]", s);
                    //     }
                    // }
                    panic!("Asset not found.");
                }
            }
        }
    }
}
