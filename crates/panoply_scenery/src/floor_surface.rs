//! Floor types, catalog entries (`*.floor.json`, D90). An entry is data; its material is built
//! here as it loads or changes, so an edit or a reload shows at once.

use bevy::{
    annotations::DisplayName,
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    pbr::ExtendedMaterial,
    platform::collections::HashMap,
    prelude::*,
};
use panoply_core::HexColor;
use panoply_exemplar::CatalogEntry;
use panoply_render::{FloorNoisyMaterial, FloorNoisyMaterialExt};

use crate::floor_region::{FloorRegion, RebuildFloorMaterials, RebuildFloorMesh};

/// A floor type: how a floor region looks, and the shape of its slab.
#[derive(Asset, Reflect, Clone, Debug)]
#[reflect(Default, @DisplayName("Floor"))]
pub struct FloorSurface {
    #[reflect(@DisplayName("Material"))]
    pub material: FloorMaterial,
    /// How far the top is above the tier's level, as a carpet is.
    #[reflect(@DisplayName("Raise"))]
    pub raise: f32,
    /// Whether the slab's sides are drawn.
    #[reflect(@DisplayName("Sides"))]
    pub sides: bool,
}

impl Default for FloorSurface {
    fn default() -> Self {
        Self {
            material: FloorMaterial::default(),
            raise: 0.,
            sides: true,
        }
    }
}

impl CatalogEntry for FloorSurface {
    const EXTENSION: &'static str = "floor.json";
}

/// How a floor is drawn. A variant's fields that a file may leave out are `#[reflect(default)]`:
/// without it, a variant missing one does not convert, and the floor's material falls back to the
/// default whole.
#[derive(Reflect, Clone, Debug)]
#[reflect(Default)]
pub enum FloorMaterial {
    /// Lit, with an optional texture tiled once a meter.
    Standard {
        /// The texture's path from the assets folder.
        #[reflect(default, @DisplayName("Texture"))]
        texture: Option<String>,
        #[reflect(default, @DisplayName("Color"))]
        color: Option<HexColor>,
        #[reflect(default, @DisplayName("Roughness"))]
        roughness: Option<f32>,
    },
    /// One color, unlit.
    Unlit {
        #[reflect(@DisplayName("Color"))]
        color: HexColor,
    },
    /// Two colors and roughnesses mixed by tiled noise.
    Noise {
        #[reflect(@DisplayName("Color"))]
        color: HexColor,
        #[reflect(@DisplayName("Second color"))]
        color_alt: HexColor,
        #[reflect(@DisplayName("Roughness"))]
        roughness: f32,
        #[reflect(@DisplayName("Second roughness"))]
        roughness_alt: f32,
    },
}

impl Default for FloorMaterial {
    fn default() -> Self {
        FloorMaterial::Standard {
            texture: None,
            color: None,
            roughness: None,
        }
    }
}

/// A floor type's material, of whichever kind it is.
#[derive(Clone)]
pub enum FloorMaterialHandle {
    Standard(Handle<StandardMaterial>),
    Noisy(Handle<FloorNoisyMaterial>),
}

/// Each loaded floor type's material.
#[derive(Resource, Default)]
pub struct FloorMaterials(HashMap<AssetId<FloorSurface>, FloorMaterialHandle>);

impl FloorMaterials {
    pub fn get(&self, surface: AssetId<FloorSurface>) -> Option<&FloorMaterialHandle> {
        self.0.get(&surface)
    }
}

/// Builds the material of each floor type that loaded or changed, and has the floors of a changed
/// one rebuilt: their material, and their slab, whose `raise` and `sides` may have changed.
pub(crate) fn build_floor_materials(
    mut events: MessageReader<AssetEvent<FloorSurface>>,
    surfaces: Res<Assets<FloorSurface>>,
    mut materials: ResMut<FloorMaterials>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut noisy_materials: ResMut<Assets<FloorNoisyMaterial>>,
    server: Res<AssetServer>,
    regions: Query<(Entity, &FloorRegion)>,
    mut commands: Commands,
) {
    for event in events.read() {
        match *event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(surface) = surfaces.get(id) else {
                    continue;
                };
                let material = surface.material.build(
                    &server,
                    &mut std_materials,
                    &mut noisy_materials,
                );
                materials.0.insert(id, material);
                if matches!(event, AssetEvent::Modified { .. }) {
                    for (entity, _) in regions.iter().filter(|(_, r)| r.surface.id() == id) {
                        commands
                            .entity(entity)
                            .insert((RebuildFloorMaterials, RebuildFloorMesh));
                    }
                }
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                materials.0.remove(&id);
            }
            AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }
}

impl FloorMaterial {
    fn build(
        &self,
        server: &AssetServer,
        std_materials: &mut Assets<StandardMaterial>,
        noisy_materials: &mut Assets<FloorNoisyMaterial>,
    ) -> FloorMaterialHandle {
        match self {
            FloorMaterial::Standard {
                texture,
                color,
                roughness,
            } => {
                let texture = texture.as_ref().map(|path| {
                    server
                        .load_builder()
                        .with_settings(|settings: &mut ImageLoaderSettings| {
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
                        })
                        .load(path.clone())
                });
                FloorMaterialHandle::Standard(std_materials.add(StandardMaterial {
                    base_color_texture: texture,
                    base_color: color.map_or(Srgba::WHITE, |color| color.0).into(),
                    perceptual_roughness: roughness.unwrap_or(1.0),
                    ..default()
                }))
            }
            FloorMaterial::Unlit { color } => {
                FloorMaterialHandle::Standard(std_materials.add(StandardMaterial {
                    base_color: color.0.into(),
                    unlit: true,
                    ..default()
                }))
            }
            FloorMaterial::Noise {
                color,
                color_alt,
                roughness,
                roughness_alt,
            } => {
                let noise: Handle<Image> = server
                    .load_builder()
                    .with_settings(|settings: &mut ImageLoaderSettings| {
                        settings.sampler = panoply_terrain::tiling_sampler();
                    })
                    .load("terrain/textures/noise.ktx2");
                FloorMaterialHandle::Noisy(noisy_materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: color.0.into(),
                        perceptual_roughness: *roughness,
                        ..default()
                    },
                    extension: FloorNoisyMaterialExt {
                        color: color.0.into(),
                        color_alt: color_alt.0.into(),
                        roughness: *roughness,
                        roughness_alt: *roughness_alt,
                        noise,
                    },
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::reflect::TypeRegistry;
    use panoply_exemplar::read_entries;
    use panoply_saga::FormulaQueue;

    use super::*;

    /// The floors as the loader reads them from `assets/catalog/floors.floor.json`.
    fn floors() -> Vec<(String, FloorSurface)> {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/catalog/floors.floor.json");
        let mut registry = TypeRegistry::new();
        registry.register::<FloorSurface>();
        read_entries(
            &std::fs::read(file).unwrap(),
            &registry,
            &mut FormulaQueue::default(),
        )
        .unwrap()
    }

    /// A variant's field the file leaves out (`color`) does not reset the others.
    #[test]
    fn a_standard_floor_keeps_its_texture_and_roughness() {
        let floors = floors();
        let (_, marble) = floors.iter().find(|(l, _)| l == "BeigeMarble").unwrap();
        let FloorMaterial::Standard {
            texture, roughness, ..
        } = &marble.material
        else {
            panic!("{:?}", marble.material);
        };
        assert_eq!(texture.as_deref(), Some("textures/beige-marble.png"));
        assert_eq!(*roughness, Some(0.35));
        assert!(marble.sides);
    }
}
