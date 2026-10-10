use bevy::prelude::*;

use crate::GroundMaterial;

/// Ambient light that varies with surface direction: `sky_color` on surfaces facing up, blending
/// to `ground_color` on surfaces facing down, as a hemisphere light does. It is much cheaper than
/// image-based lighting, and gives the terrain the look of the original three.js version.
///
/// The ground shader applies it per pixel. Other materials get `GlobalAmbientLight`, set to the
/// average of the two colors. Reflected, so it can be tuned at runtime over BRP.
#[derive(Resource, Reflect, Clone, Debug, PartialEq)]
#[reflect(Resource)]
pub struct HemisphereLight {
    /// Light from above.
    pub sky_color: Color,
    /// Light bounced up from the ground.
    pub ground_color: Color,
    /// Intensity, in the same units as `GlobalAmbientLight::brightness` (cd/m²).
    pub brightness: f32,
}

impl Default for HemisphereLight {
    fn default() -> Self {
        Self {
            sky_color: Srgba::rgb(0.75, 0.85, 1.0).into(),
            ground_color: Srgba::rgb(0.5, 0.45, 0.35).into(),
            brightness: 200.,
        }
    }
}

impl HemisphereLight {
    /// Sky and ground colors with brightness folded in, as the shader uniform wants them.
    pub fn uniform(&self) -> [LinearRgba; 2] {
        [
            self.sky_color.to_linear() * self.brightness,
            self.ground_color.to_linear() * self.brightness,
        ]
    }
}

/// Copies the hemisphere light into ground materials that are out of date (including ones
/// created since the last change), and keeps `GlobalAmbientLight` at the average color.
pub(crate) fn sync_hemisphere_light(
    hemisphere: Res<HemisphereLight>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut ground_materials: ResMut<Assets<GroundMaterial>>,
) {
    let uniform = hemisphere.uniform();
    let stale: Vec<_> = ground_materials
        .iter()
        .filter(|(_, material)| material.hemisphere != uniform)
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut material) = ground_materials.get_mut(id) {
            material.hemisphere = uniform;
        }
    }

    if hemisphere.is_changed() {
        let average =
            (hemisphere.sky_color.to_linear() + hemisphere.ground_color.to_linear()) * 0.5;
        ambient.color = average.into();
        ambient.brightness = hemisphere.brightness;
    }
}
