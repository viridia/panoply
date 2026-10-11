use bevy::prelude::*;

/// Color and strength of every realm's sun. Kept low next to the hemisphere light, as in the
/// three.js version (sun 0.4, hemisphere 0.5), so shadows stay soft and the painted textures read
/// close to their own colors. Reflected, so it can be tuned at runtime over BRP.
///
/// Three.js (0.152, legacy light units) scaled light intensities by π, so 0.4 there is
/// 0.4 · π / exposure here; at Bevy's default exposure (EV100 9.7, about 1/1000) that is about
/// 1250 lux. The default is 1.4 times that, as is `HemisphereLight`'s, which reads closer to the
/// old engine's screenshots.
#[derive(Resource, Reflect, Clone, Debug, PartialEq)]
#[reflect(Resource)]
pub struct Sunlight {
    pub color: Color,
    /// Illuminance in lux, as `DirectionalLight::illuminance`.
    pub illuminance: f32,
}

impl Default for Sunlight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            illuminance: 1750.,
        }
    }
}

/// Marker for a realm's sun, the directional light that `Sunlight` controls.
#[derive(Component, Default)]
pub struct RealmSun;

/// Copies `Sunlight` into realm suns that differ from it (including newly spawned ones).
pub(crate) fn sync_sunlight(
    sunlight: Res<Sunlight>,
    mut suns: Query<&mut DirectionalLight, With<RealmSun>>,
) {
    for mut light in &mut suns {
        if light.color != sunlight.color || light.illuminance != sunlight.illuminance {
            light.color = sunlight.color;
            light.illuminance = sunlight.illuminance;
        }
    }
}
