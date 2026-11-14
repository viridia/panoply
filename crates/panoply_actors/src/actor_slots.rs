//! An actor's slots, read from its skin's character file rather than listed by hand. A feature is
//! a scene of the file made from a top-level object whose extras set `feature`. A
//! color slot is a material that the body scene or a feature scene uses and whose extras set
//! `color_slot`; every other material keeps its own color.

use std::collections::BTreeSet;

use bevy::{
    gltf::{Gltf, GltfExtras, GltfMaterialExtras, GltfMaterialName},
    prelude::*,
    world_serialization::WorldAsset,
};
use panoply_exemplar::{annotations::PropertyMapKeys, Exemplar};

use super::actor_aspect::{Colors, Features, Skin};

/// Whether the glTF extras `extras` set `key`, as `true` or, as Blender writes a boolean custom
/// property, a number other than 0.
fn extra_flag(extras: &str, key: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(extras) else {
        return false;
    };
    match value.get(key) {
        Some(serde_json::Value::Bool(flag)) => *flag,
        Some(serde_json::Value::Number(number)) => number.as_f64().is_some_and(|n| n != 0.0),
        _ => false,
    }
}

/// Whether `scene`, the scene of a character file named `name`, is a feature: the pipeline makes
/// each top-level object a scene of its name, so the node of that name is marked. The file's own
/// scene, which holds every object, features included, is not.
pub fn is_feature_scene(scene: &World, name: &str) -> bool {
    let Some(mut query) = scene.try_query::<(&Name, &GltfExtras)>() else {
        return false;
    };
    query
        .iter(scene)
        .any(|(node, extras)| node.as_str() == name && extra_flag(&extras.value, "feature"))
}

/// The names of `gltf`'s feature scenes, in order.
fn feature_scenes(gltf: &Gltf, scenes: &Assets<WorldAsset>) -> Vec<String> {
    let mut names: Vec<String> = gltf
        .named_scenes
        .iter()
        .filter(|(name, handle)| {
            scenes.get(*handle).is_some_and(|scene| is_feature_scene(&scene.world, name))
        })
        .map(|(name, _)| name.to_string())
        .collect();
    names.sort();
    names
}

/// Adds to `slots` the materials of `scene`'s meshes that are color slots.
fn color_slot_materials(scene: &World, slots: &mut BTreeSet<String>) {
    let Some(mut query) = scene.try_query::<(&GltfMaterialName, &GltfMaterialExtras)>() else {
        return;
    };
    for (name, extras) in query.iter(scene) {
        if extra_flag(&extras.value, "color_slot") {
            slots.insert(name.0.clone());
        }
    }
}

/// The character file of `exemplar`'s skin and the name of its body scene, the file loaded and its
/// handle put in `held`: `Some(None)` if it has no skin or the file failed to load, `None` while
/// it loads.
fn skin_model<'w>(
    world: &'w World,
    exemplar: AssetId<Exemplar>,
    held: &mut Vec<UntypedHandle>,
) -> Option<Option<(&'w Gltf, &'w str)>> {
    let exemplars = world.resource::<Assets<Exemplar>>();
    let Some((file, scene)) = Exemplar::find_aspect::<Skin>(exemplars, exemplar)
        .and_then(|skin| skin.0.split_once('#'))
    else {
        return Some(None);
    };
    let server = world.resource::<AssetServer>();
    let handle = server.load::<Gltf>(file.to_string());
    held.push(handle.clone().untyped());
    if server.load_state(&handle).is_failed() {
        return Some(None);
    }
    let gltf = world.resource::<Assets<Gltf>>().get(&handle)?;
    Some(Some((gltf, scene)))
}

/// An actor's feature slots: its skin file's feature scenes.
impl PropertyMapKeys for Features {
    fn property_map_keys(
        &self,
        world: &World,
        exemplar: AssetId<Exemplar>,
        held: &mut Vec<UntypedHandle>,
    ) -> Option<Vec<String>> {
        let Some((gltf, _)) = skin_model(world, exemplar, held)? else {
            return Some(Vec::new());
        };
        Some(feature_scenes(gltf, world.resource::<Assets<WorldAsset>>()))
    }
}

/// An actor's color slots: the materials marked `color_slot` that its body and feature scenes
/// use, in order.
impl PropertyMapKeys for Colors {
    fn property_map_keys(
        &self,
        world: &World,
        exemplar: AssetId<Exemplar>,
        held: &mut Vec<UntypedHandle>,
    ) -> Option<Vec<String>> {
        let Some((gltf, body)) = skin_model(world, exemplar, held)? else {
            return Some(Vec::new());
        };
        let scenes = world.resource::<Assets<WorldAsset>>();
        let mut slots = BTreeSet::new();
        let features = feature_scenes(gltf, scenes);
        for name in std::iter::once(body).chain(features.iter().map(String::as_str)) {
            let scene = gltf.named_scenes.get(name).and_then(|handle| scenes.get(handle));
            if let Some(scene) = scene {
                color_slot_materials(&scene.world, &mut slots);
            }
        }
        Some(slots.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::extra_flag;

    #[test]
    fn a_flag_is_a_true_or_a_number_other_than_zero() {
        assert!(extra_flag(r#"{"feature": true}"#, "feature"));
        assert!(extra_flag(r#"{"feature": 1}"#, "feature"));
        assert!(!extra_flag(r#"{"feature": 0}"#, "feature"));
        assert!(!extra_flag(r#"{"outline": 0.01}"#, "feature"));
        assert!(!extra_flag("not json", "feature"));
    }
}
