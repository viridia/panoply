use crate::schematic::{Aspect, DetachAspect, ReflectAspect, SimpleDetachAspect};
use bevy::{
    prelude::*,
    render::texture::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
};

/// Floor surface aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct StdFloorSurface {
    /// Texture to use for this floor
    texture: Option<String>,

    color: Option<String>,

    #[reflect(ignore)]
    pub(crate) material: Handle<StandardMaterial>,

    /// Surface roughness
    roughness: Option<f32>,
    // water_current_x: Option<f32>,
    // water_current_y: Option<f32>,
}

impl Aspect for StdFloorSurface {
    fn name(&self) -> &str {
        "StdFloorSurface"
    }

    fn can_attach(&self, meta_type: crate::schematic::InstanceType) -> bool {
        meta_type == crate::schematic::InstanceType::Floor
    }

    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn load_dependencies(&mut self, label: &str, load_context: &mut bevy::asset::LoadContext) {
        println!("Loading material: {}.StdFloorSurface.Material", label);
        self.material =
            load_context.labeled_asset_scope(format!("{}.StdFloorSurface.Material", label), |lc| {
                let mut material = StandardMaterial {
                    perceptual_roughness: self.roughness.unwrap_or(1.0),
                    ..default()
                };
                if let Some(color) = &self.color {
                    material.base_color = Color::hex(color).unwrap();
                } else if let Some(texture) = &self.texture {
                    material.base_color_texture = Some(lc.load_with_settings(
                        texture,
                        |settings: &mut ImageLoaderSettings| {
                            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                label: Some("Floor Region".to_string()),
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                address_mode_w: ImageAddressMode::Repeat,
                                mag_filter: ImageFilterMode::Linear,
                                min_filter: ImageFilterMode::Linear,
                                mipmap_filter: ImageFilterMode::Linear,
                                ..default()
                            });
                        },
                    ));
                }
                // material.base_color_texture_transform = TextureTransform {
                //     offset: Vec2::new(0.0, 0.0),
                //     scale: Vec2::new(1.0, 1.0),
                //     rotation: 0.0,
                // };
                material
            });
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        entity.insert(self.clone());
        &FLOOR_SURFACE_REMOVER
    }
}

static FLOOR_SURFACE_REMOVER: SimpleDetachAspect<StdFloorSurface> =
    SimpleDetachAspect::<StdFloorSurface>::new();

/// Floor surface aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct NoiseFloorSurface {
    /// Noise base color
    color_base: Option<String>,

    /// Noice accent color
    color_accent: Option<String>,

    #[reflect(ignore)]
    pub(crate) material_handle: Handle<StandardMaterial>,

    roughness: Option<f32>,
}

impl Aspect for NoiseFloorSurface {
    fn name(&self) -> &str {
        "NoiseFloorSurface"
    }

    fn can_attach(&self, meta_type: crate::schematic::InstanceType) -> bool {
        meta_type == crate::schematic::InstanceType::Floor
    }

    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn load_dependencies(&mut self, label: &str, load_context: &mut bevy::asset::LoadContext) {
        println!("Loading material: {}.StdFloorSurface.Material", label);
        self.material_handle =
            load_context.labeled_asset_scope(format!("{}.StdFloorSurface.Material", label), |lc| {
                let mut material = StandardMaterial {
                    perceptual_roughness: self.roughness.unwrap_or(1.0),
                    ..default()
                };
                // if let Some(color) = &self.color {
                //     material.base_color = Color::hex(color).unwrap();
                // }
                material
            });
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        entity.insert(self.clone());
        &NOISE_FLOOR_SURFACE_REMOVER
    }
}

static NOISE_FLOOR_SURFACE_REMOVER: SimpleDetachAspect<NoiseFloorSurface> =
    SimpleDetachAspect::<NoiseFloorSurface>::new();

/// Floor surface aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct FloorGeometry {
    /// How far up the floor should be raised or lowered.
    raise: f32,

    /// Whether to render the sides of this floor.
    sides: Option<bool>,
}

impl Aspect for FloorGeometry {
    fn name(&self) -> &str {
        "FloorGeometry"
    }

    fn can_attach(&self, meta_type: crate::schematic::InstanceType) -> bool {
        meta_type == crate::schematic::InstanceType::Floor
    }

    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        entity.insert(self.clone());
        &FLOOR_GEOMETRY_REMOVER
    }
}

static FLOOR_GEOMETRY_REMOVER: SimpleDetachAspect<FloorGeometry> =
    SimpleDetachAspect::<FloorGeometry>::new();

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

    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        entity.insert(self.clone());
        &FLOOR_NAV_REMOVER
    }
}

static FLOOR_NAV_REMOVER: SimpleDetachAspect<FloorNav> = SimpleDetachAspect::<FloorNav>::new();
