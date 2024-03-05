use crate::schematic::{Aspect, DetachAspect, ReflectAspect, SimpleDetachAspect};
use bevy::{
    prelude::*,
    render::texture::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    utils::HashMap,
};

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
    pub(crate) material_handle: Handle<StandardMaterial>,

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

    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn load_dependencies(&mut self, label: &str, load_context: &mut bevy::asset::LoadContext) {
        println!("Loading material: {}.FloorSurface.Material", label);
        self.material_handle =
            load_context.labeled_asset_scope(format!("{}.FloorSurface.Material", label), |lc| {
                let mut material = StandardMaterial {
                    perceptual_roughness: self.roughness.unwrap_or(1.0),
                    ..default()
                };
                if let Some(color) = &self.color {
                    material.base_color = Color::hex(color).unwrap();
                } else if let Some(texture) = &self.texture {
                    self.texture_handle = Some(lc.load_with_settings(
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
                    material.base_color_texture = self.texture_handle.clone();
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

static FLOOR_SURFACE_REMOVER: SimpleDetachAspect<FloorSurface> =
    SimpleDetachAspect::<FloorSurface>::new();

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
