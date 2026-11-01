use bevy::{platform::collections::HashMap, prelude::*};
use panoply_exemplar::*;
use serde::{Deserialize, Serialize};

use panoply_core::HexColor;

use super::ACTOR_TYPE;

/// The species' skeleton and the file holding its animation clips, each a `file#Label` path.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct Armature {
    pub armature: String,
    #[serde(default)]
    pub animations: Option<String>,
}

impl Aspect for Armature {
    fn name(&self) -> &str {
        "Armature"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Armature> = RemoveComponent::<Armature>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// The actor's body: a `file#Scene` path to a named scene of a character glTF, which holds the
/// body mesh and the skeleton it is bound to.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct Skin(pub String);

impl Aspect for Skin {
    fn name(&self) -> &str {
        "Skin"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Skin> = RemoveComponent::<Skin>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Color assignments for this actor.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct Colors(pub HashMap<String, HexColor>);

impl Aspect for Colors {
    fn name(&self) -> &str {
        "Colors"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Colors> = RemoveComponent::<Colors>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Available color slots for this actor.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct ColorSlots(pub Vec<String>);

impl Aspect for ColorSlots {
    fn name(&self) -> &str {
        "ColorSlots"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<ColorSlots> = RemoveComponent::<ColorSlots>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Enabled features for this actor.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct Features(pub HashMap<String, bool>);

impl Aspect for Features {
    fn name(&self) -> &str {
        "Features"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Features> = RemoveComponent::<Features>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Available feature slots for this actor.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct FeatureSlots(pub Vec<String>);

impl Aspect for FeatureSlots {
    fn name(&self) -> &str {
        "FeatureSlots"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<FeatureSlots> = RemoveComponent::<FeatureSlots>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
