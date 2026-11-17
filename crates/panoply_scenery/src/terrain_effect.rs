//! Terrain effects, catalog entries (`*.terrainfx.json`, D90): what a precinct's terrain effect
//! map paints onto the ground of its parcels.

use bevy::{annotations::DisplayName, prelude::*};
use panoply_exemplar::CatalogEntry;
use panoply_terrain::{TerrainFxVertexAttr, TerrainOptions, TerrainTypes};

/// A terrain effect: a change to the ground's surface, or a hole in it.
#[derive(Asset, Reflect, Clone, Debug)]
#[reflect(Default, @DisplayName("Terrain effect"))]
pub enum TerrainEffect {
    /// Ground painted with other terrain types, and raised or lowered.
    Surface {
        #[reflect(default, @DisplayName("Effect"))]
        effect: TerrainTypes,
        #[reflect(default, @DisplayName("Strength"))]
        effect_strength: f32,
        #[reflect(default, @DisplayName("Elevation"))]
        elevation: f32,
        /// Whether the effect runs on along x into a neighbour's, rather than stopping at each
        /// tile, as a road does.
        #[reflect(default, @DisplayName("Continuous x"))]
        continuous_x: bool,
        /// As `continuous_x`, along the map's y, the world's z.
        #[reflect(default, @DisplayName("Continuous y"))]
        continuous_y: bool,
    },
    /// No ground, as at the mouth of a cave or a mine.
    Hole,
}

impl Default for TerrainEffect {
    fn default() -> Self {
        TerrainEffect::Surface {
            effect: TerrainTypes::empty(),
            effect_strength: 0.,
            elevation: 0.,
            continuous_x: false,
            continuous_y: false,
        }
    }
}

impl CatalogEntry for TerrainEffect {
    const EXTENSION: &'static str = "terrainfx.json";
}

impl TerrainEffect {
    /// The effect as the ground mesh's vertices carry it.
    pub fn vertex_attr(&self) -> TerrainFxVertexAttr {
        let mut attr = TerrainFxVertexAttr::default();
        match *self {
            TerrainEffect::Surface {
                effect,
                effect_strength,
                elevation,
                continuous_x,
                continuous_y,
            } => {
                attr.effect = effect;
                attr.effect_strength = effect_strength;
                attr.elevation = elevation;
                if continuous_x {
                    attr.options |= TerrainOptions::ContinuousX;
                }
                if continuous_y {
                    attr.options |= TerrainOptions::ContinuousY;
                }
            }
            TerrainEffect::Hole => attr.options |= TerrainOptions::Hole,
        }
        attr
    }
}

#[cfg(test)]
mod tests {
    use bevy::reflect::TypeRegistry;
    use panoply_exemplar::read_entries;
    use panoply_saga::FormulaQueue;

    use super::*;

    /// The effects as the loader reads them from `assets/catalog/terrain-effects.terrainfx.json`.
    fn effects() -> Vec<(String, TerrainEffect)> {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/catalog/terrain-effects.terrainfx.json");
        let mut registry = TypeRegistry::new();
        registry.register::<TerrainEffect>();
        read_entries(
            &std::fs::read(file).unwrap(),
            &registry,
            &mut FormulaQueue::default(),
        )
        .unwrap()
    }

    fn attr(label: &str) -> TerrainFxVertexAttr {
        let effects = effects();
        let (_, effect) = effects.iter().find(|(l, _)| l == label).unwrap();
        effect.vertex_attr()
    }

    /// A field the file leaves out is at its default, and the others are kept.
    #[test]
    fn a_surface_reads_its_fields() {
        let road = attr("Road.Med");
        assert_eq!(road.effect, TerrainTypes::Cobbles);
        assert_eq!(road.effect_strength, 0.75);
        assert_eq!(road.elevation, -0.1);
        assert_eq!(
            road.options,
            TerrainOptions::ContinuousX | TerrainOptions::ContinuousY
        );
        let flat = attr("Road.Flat");
        assert_eq!(flat.elevation, 0.);
        let grave = attr("Grave.X");
        assert_eq!(grave.options, TerrainOptions::ContinuousX);
    }

    #[test]
    fn the_hole_is_a_hole() {
        assert_eq!(attr("Hole").options, TerrainOptions::Hole);
    }

    /// Saving writes each effect as the file has it, so an unedited entry merges as unchanged.
    #[test]
    fn effects_serialize_as_the_file_has_them() {
        let mut registry = TypeRegistry::new();
        registry.register::<TerrainEffect>();
        let file: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../assets/catalog/terrain-effects.terrainfx.json"),
            )
            .unwrap(),
        )
        .unwrap();
        for (label, effect) in effects() {
            let serializer = bevy::reflect::serde::TypedReflectSerializer::new(
                effect.as_partial_reflect(),
                &registry,
            );
            let mut json = serde_json::to_value(serializer).unwrap();
            // Fields at their defaults are left out of the file.
            if let Some(fields) = json.get_mut("Surface").and_then(|s| s.as_object_mut()) {
                fields.retain(|_, v| !(v == &serde_json::json!(0.0) || v == &serde_json::json!(false)));
            }
            let mut expected = file[&label].clone();
            if let Some(fields) = expected.get_mut("Surface").and_then(|s| s.as_object_mut()) {
                for value in fields.values_mut() {
                    if let Some(n) = value.as_f64() {
                        *value = serde_json::json!(n as f32 as f64);
                    }
                }
            }
            assert_eq!(json, expected, "{label}");
        }
    }
}
