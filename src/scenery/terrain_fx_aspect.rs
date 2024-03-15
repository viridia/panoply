use crate::terrain::TerrainTypes;
use bevy::prelude::*;
use panoply_exemplar::*;

use super::TERRAIN_FX_TYPE;

/// Description of a terrain effect. Terrain effects are a property of precincts, but are
/// applied to terrain parcels.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct TerrainEffect {
    pub effect: TerrainTypes,
    pub effect_strength: Option<f32>,
    pub elevation: Option<f32>,
    pub continuous_x: Option<bool>,
    pub continuous_y: Option<bool>,
}

impl Aspect for TerrainEffect {
    fn name(&self) -> &str {
        "TerrainEffect"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == TERRAIN_FX_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<TerrainEffect> = RemoveComponent::<TerrainEffect>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Indicates a hole in the terrain.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct TerrainHole;

impl Aspect for TerrainHole {
    fn name(&self) -> &str {
        "TerrainHole"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == TERRAIN_FX_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<TerrainHole> = RemoveComponent::<TerrainHole>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
