// mod model_loader;
mod warmup;

use bevy::{
    gltf::GltfMaterialExtras, light::NotShadowCaster, mesh::skinning::SkinnedMesh,
    platform::collections::HashMap, prelude::*,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::materials::{BlackMaterialHandle, FlameMaterialHandle, OutlineMaterialHandle};

pub struct ModelsPlugin;

impl Plugin for ModelsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(warmup::WarmupPlugin)
            .add_systems(Update, process_material_extras);
    }
}

#[derive(Debug, Clone, Deserialize)]
struct MaterialOptions {
    outline: Option<f32>,

    #[serde(default, deserialize_with = "deserialize_bool")]
    interact: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_bool")]
    black: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_bool")]
    unlit: Option<bool>,

    // #[serde(default, deserialize_with = "deserialize_bool")]
    flame: Option<Value>,

    #[serde(default, deserialize_with = "deserialize_bool")]
    portalglow: Option<bool>,

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

fn process_material_extras(
    mut commands: Commands,
    r_outline: Res<OutlineMaterialHandle>,
    r_black: Res<BlackMaterialHandle>,
    r_flame: Res<FlameMaterialHandle>,
    q_materials: Query<
        (Entity, &Mesh3d, Option<&SkinnedMesh>, &GltfMaterialExtras),
        (
            With<MeshMaterial3d<StandardMaterial>>,
            Added<GltfMaterialExtras>,
        ),
    >,
) {
    for (entity, mesh, skin, extras) in q_materials.iter() {
        // println!("material extras: {:?}", extras);
        let options = serde_json::from_str::<MaterialOptions>(&extras.value);
        // println!("material options: {:?}", options);
        if let Ok(options) = options {
            if options.outline.is_some() {
                // The outline is an inverted hull drawn over the same mesh. It is a child entity
                // rather than a second material on this one: an entity with two materials breaks
                // batching in the shadow and prepass pipelines (a wgpu validation error). It
                // inherits the model's render layers through `Propagate<RenderLayers>`.
                let mut outline = commands.spawn((
                    Name::new("Outline"),
                    Mesh3d(mesh.0.clone()),
                    MeshMaterial3d(r_outline.0.clone()),
                    NotShadowCaster,
                    ChildOf(entity),
                ));
                // A skinned mesh's pipeline expects joint bindings, so the hull needs the skin too.
                if let Some(skin) = skin {
                    outline.insert(skin.clone());
                }
            } else if options.black.unwrap_or(false) || options.unlit.unwrap_or(false) {
                commands
                    .entity(entity)
                    // TODO: Remove standard material?
                    .insert(MeshMaterial3d(r_black.0.clone()));
            } else if options.flame.is_some() {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert(MeshMaterial3d(r_flame.0.clone()));
            } else if !options.extra.is_empty() {
                // warn!("Unknown material option: {:?}", options.extra);
                // println!("material extras: {:?}", extras);
            }
        }
    }
}

// Because Blender is inconsistent about how it exports booleans: true, "true" or 1.
fn deserialize_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;
    match value {
        Value::Bool(b) => Ok(Some(b)),
        Value::String(s) => match s.to_lowercase().as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => Err(serde::de::Error::custom("Invalid string for boolean")),
        },
        Value::Number(n) => Ok(Some(n.as_f64() != Some(0.))),
        Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("Invalid type for boolean")),
    }
}
