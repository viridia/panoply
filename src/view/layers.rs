use bevy::{prelude::*, render::view::RenderLayers};

#[derive(Default, Resource)]
pub struct ReservedLayers(RenderLayers);

impl ReservedLayers {
    pub fn next_unused(&mut self) -> usize {
        let mut layer: usize = 1;
        while self.0.intersects(&RenderLayers::layer(layer)) {
            layer += 1;
        }
        self.0 = self.0.clone().with(layer);
        layer
    }

    pub fn release(&mut self, layer: usize) {
        self.0 = self.0.clone().without(layer);
    }
}
