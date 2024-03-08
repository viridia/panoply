use crate::schematic::{Aspect, DetachAspect, InstanceType, ReflectAspect, SimpleDetachAspect};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::TypeId;

#[derive(Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainEffectType {
    #[default]
    Earth,
    Soil,
    Path,
    Cobbles,
    Road,
    Stone,
}

#[derive(Debug, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TerrainEffectSet {
    SingleEffect(TerrainEffectType),
    MultipleEffects(Vec<TerrainEffectType>),
}

/// Description of a terrain effect. Terrain effects are a property of precincts, but are
/// applied to terrain parcels.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct TerrainEffect {
    effect: Option<TerrainEffectSet>,
    effect_strength: Option<f32>,
    elevation: Option<f32>,
    continuous_x: Option<bool>,
    continuous_y: Option<bool>,
}

impl Aspect for TerrainEffect {
    fn name(&self) -> &str {
        "TerrainEffect"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::TerrainFx
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<TerrainEffect> =
            SimpleDetachAspect::<TerrainEffect>::new();
        entity.insert(self.clone());
        &DETACH
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
        meta_type == InstanceType::TerrainFx
    }

    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<TerrainHole> = SimpleDetachAspect::<TerrainHole>::new();
        entity.insert(self.clone());
        &DETACH
    }
}
