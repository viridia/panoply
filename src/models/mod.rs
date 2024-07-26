// mod model_loader;

use bevy::{
    ecs::world::Command,
    gltf::GltfMaterialExtras,
    pbr::ExtendedMaterial,
    prelude::*,
    render::{render_resource::Face, view::RenderLayers},
    scene::SceneInstance,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::materials::{OutlineMaterial, OutlineMaterialExtension};

/// A component that indicates that we want to propagate render layers to all descendants.
#[derive(Debug, Clone, Copy, Component)]
pub struct PropagateRenderLayers;

pub struct ModelsPlugin;

#[derive(Debug, Clone, Resource, Default)]
struct OutlineMaterialHandle(Handle<OutlineMaterial>);

#[derive(Debug, Clone, Resource, Default)]
struct BlackMaterialHandle(Handle<StandardMaterial>);

impl Plugin for ModelsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<OutlineMaterialHandle>()
            .init_resource::<BlackMaterialHandle>()
            .add_systems(Startup, init_outline)
            .add_systems(Update, (copy_model_render_layers, process_material_extras));
    }
}

fn init_outline(
    mut materials: ResMut<Assets<OutlineMaterial>>,
    mut r_outline: ResMut<OutlineMaterialHandle>,
) {
    r_outline.0 = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..Default::default()
        },
        extension: OutlineMaterialExtension { width: 0.015 },
    });
}

fn init_black(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut r_black: ResMut<BlackMaterialHandle>,
) {
    r_black.0 = materials.add(StandardMaterial {
        base_color: Srgba::BLACK.into(),
        // unlit: true,
        ..Default::default()
    });
}

pub fn copy_model_render_layers(
    mut commands: Commands,
    q_models_added: Query<(Entity, &RenderLayers, &PropagateRenderLayers), Added<SceneInstance>>,
    q_children: Query<&Children>,
) {
    for (entity, layers, _) in q_models_added.iter() {
        for descendant in q_children.iter_descendants(entity) {
            commands.add(SafeInsertLayers {
                layers: layers.clone(),
                target: descendant,
            });
        }
    }
}

struct SafeInsertLayers {
    layers: RenderLayers,
    target: Entity,
}

impl Command for SafeInsertLayers {
    fn apply(self, world: &mut World) {
        // Check if entity exists.
        if let Some(mut entity) = world.get_entity_mut(self.target) {
            entity.insert(self.layers.clone());
        }
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

    #[serde(default, deserialize_with = "deserialize_bool")]
    flame: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_bool")]
    portalglow: Option<bool>,
}

fn process_material_extras(
    mut commands: Commands,
    r_outline: Res<OutlineMaterialHandle>,
    r_black: Res<BlackMaterialHandle>,
    q_materials: Query<
        (Entity, &Handle<StandardMaterial>, &GltfMaterialExtras),
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
                commands.entity(entity).insert(r_outline.0.clone());
            } else if options.black.unwrap_or(false) || options.unlit.unwrap_or(false) {
                // commands.entity(entity).remove::<Handle<StandardMaterial>>();
                commands.entity(entity).insert(r_black.0.clone());
            } else {
                // warn!("Unknown material option: {:?}", extras);
                // println!("material extras: {:?}", extras);
            }
        }
    }
}

// Because Blender is inconsistent about how it exports booleans (changed in recent blender version).
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
        Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("Invalid type for boolean")),
    }
}
