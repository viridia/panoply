use bevy::{camera::visibility::RenderLayers, prelude::*};

/// The layer of portal apertures, the surfaces that show a portal camera's image. The primary
/// camera renders it, beside its realm's layer; portal cameras do not, since an aperture drawn in
/// its own camera's view would read the image that camera is writing. Above the realms' layers,
/// which [`ReservedLayers`] hands out from 1.
pub const PORTAL_APERTURE_LAYER: usize = 31;

#[derive(Default, Resource)]
pub struct ReservedLayers(RenderLayers);

impl ReservedLayers {
    pub fn next_unused(&mut self) -> usize {
        let mut layer: usize = 1;
        while layer == PORTAL_APERTURE_LAYER || self.0.intersects(&RenderLayers::layer(layer)) {
            layer += 1;
        }
        self.0 = self.0.clone().with(layer);
        layer
    }

    pub fn release(&mut self, layer: usize) {
        self.0 = self.0.clone().without(layer);
    }
}
