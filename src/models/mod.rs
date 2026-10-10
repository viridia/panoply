// mod model_loader;

use bevy::{
    camera::visibility::RenderLayers, gltf::GltfMaterialExtras, platform::collections::HashMap,
    prelude::*, world_serialization::WorldInstanceReady,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::materials::{BlackMaterialHandle, FlameMaterialHandle, OutlineMaterialHandle};

/// A component that indicates that we want to propagate render layers to all descendants.
#[derive(Debug, Clone, Copy, Component)]
pub struct PropagateRenderLayers;

pub struct ModelsPlugin;

impl Plugin for ModelsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(copy_model_render_layers)
            .add_systems(Update, process_material_extras);
    }
}

/// When a model's world instance has finished spawning, copy its render layers to all
/// descendants.
pub fn copy_model_render_layers(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    q_models: Query<&RenderLayers, With<PropagateRenderLayers>>,
    q_children: Query<&Children>,
) {
    let Ok(layers) = q_models.get(ready.entity) else {
        return;
    };
    for descendant in q_children.iter_descendants(ready.entity) {
        commands.entity(descendant).try_insert(layers.clone());
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
        (
            Entity,
            &mut MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
        ),
        Added<GltfMaterialExtras>,
    >,
) {
    for (entity, _material, extras) in q_materials.iter() {
        // println!("material extras: {:?}", extras);
        let options = serde_json::from_str::<MaterialOptions>(&extras.value);
        // println!("material options: {:?}", options);
        if let Ok(options) = options {
            if options.outline.is_some() {
                // println!("outline material");
                // Add outline material, but keep existing material as well.
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(r_outline.0.clone()));
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
