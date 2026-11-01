use bevy::{
    gltf::GltfMaterialName, platform::collections::HashMap, prelude::*,
    world_serialization::WorldInstanceReady,
};

use super::{actor_aspect::Colors, ActorBody, ActorFeature};

/// Recolored clones of slot materials, by (material, sRGB color). Holds ids, not handles, so a
/// clone is freed once no actor uses it, and made again if needed.
#[derive(Resource, Default)]
pub struct ActorMaterials(HashMap<(AssetId<StandardMaterial>, [u8; 4]), AssetId<StandardMaterial>>);

/// Applies the actor's `Colors` to its body or a feature once the scene has spawned: each mesh
/// whose material is named for a color slot gets a clone of that material with the slot's color
/// (D28).
pub fn color_actor_part(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    parts: Query<&ChildOf, Or<(With<ActorBody>, With<ActorFeature>)>>,
    colors: Query<&Colors>,
    children: Query<&Children>,
    meshes: Query<(&GltfMaterialName, &MeshMaterial3d<StandardMaterial>)>,
    mut cache: ResMut<ActorMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(actor) = parts.get(ready.entity) else {
        return;
    };
    let Ok(colors) = colors.get(actor.parent()) else {
        return;
    };
    for entity in children.iter_descendants(ready.entity) {
        let Ok((slot, material)) = meshes.get(entity) else {
            continue;
        };
        let Some(color) = colors.0.get(&slot.0) else {
            continue;
        };
        let key = (material.id(), color.to_u8_array());
        let cached = cache
            .0
            .get(&key)
            .and_then(|id| materials.get_strong_handle(*id));
        let handle = match cached {
            Some(handle) => handle,
            None => {
                let Some(source) = materials.get(material.id()) else {
                    continue;
                };
                let mut clone = source.clone();
                clone.base_color = Color::Srgba(color.0);
                let handle = materials.add(clone);
                cache.0.insert(key, handle.id());
                handle
            }
        };
        commands.entity(entity).insert(MeshMaterial3d(handle));
    }
}
