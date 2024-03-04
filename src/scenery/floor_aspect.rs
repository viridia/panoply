use std::any::Any;

use bevy::{ecs::component::ComponentId, prelude::*, utils::HashMap};

use crate::schematic::{Aspect, ReflectAspect};

/// Floor surface aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct FloorSurface {
    /// Texture to use for this floor
    texture: Option<String>,

    #[reflect(ignore)]
    texture_handle: Option<Handle<Image>>,

    color: Option<String>,
    // color: Option<Srgba>,
    // colors: Record<string, string>,
    colors: HashMap<String, String>,
    color_slots: Vec<String>,
    material: Option<String>,

    #[reflect(ignore)]
    material_handle: Option<Handle<StandardMaterial>>,

    roughness: Option<f32>,
    raise: f32,

    /// Whether to render the sides of this floor.
    sides: Option<bool>,
    // water_current_x: Option<f32>,
    // water_current_y: Option<f32>,
}

impl Aspect for FloorSurface {
    fn name(&self) -> &str {
        "FloorSurface"
    }

    fn can_attach(&self, meta_type: crate::schematic::InstanceType) -> bool {
        meta_type == crate::schematic::InstanceType::Floor
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn component_id(&self, world: &mut World) -> ComponentId {
        world.init_component::<Self>()
    }
}

/// Floor navigation aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct FloorNav {
    blocked: bool,
}

impl Aspect for FloorNav {
    fn name(&self) -> &str {
        "FloorNav"
    }

    fn can_attach(&self, meta_type: crate::schematic::InstanceType) -> bool {
        meta_type == crate::schematic::InstanceType::Floor
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn component_id(&self, world: &mut World) -> ComponentId {
        world.init_component::<Self>()
    }
}
