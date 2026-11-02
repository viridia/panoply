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

/// Renames the clips the engine asks for to the species' own, where they differ: humanoids share
/// one clips file whose gaits are named by gender (`"Walk": "WalkMale"`).
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct ClipNames(pub HashMap<String, String>);

impl Aspect for ClipNames {
    fn name(&self) -> &str {
        "ClipNames"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<ClipNames> = RemoveComponent::<ClipNames>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// The actor's collision capsule (R4), in meters: its whole height, and its radius.
#[derive(Component, Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct CollisionSize {
    pub height: f32,
    pub radius: f32,
}

impl Default for CollisionSize {
    fn default() -> Self {
        Self {
            height: 1.0,
            radius: 0.5,
        }
    }
}

impl CollisionSize {
    /// The capsule an actor gets: this size, times its `ActorScale`.
    pub fn scaled_by(&self, scale: Option<&ActorScale>) -> Self {
        let scale = scale.map_or(1.0, |scale| scale.0);
        Self {
            height: self.height * scale,
            radius: self.radius * scale,
        }
    }
}

impl Aspect for CollisionSize {
    fn name(&self) -> &str {
        "CollisionSize"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<CollisionSize> = RemoveComponent::<CollisionSize>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Draws the actor's body this many times its modelled size, and scales its `CollisionSize` to
/// match: one species' model serves a large or small individual.
#[derive(Component, Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct ActorScale(pub f32);

impl Default for ActorScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Aspect for ActorScale {
    fn name(&self) -> &str {
        "ActorScale"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<ActorScale> = RemoveComponent::<ActorScale>::new();
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

/// Items the actor holds, as item exemplar paths, each shown on the bone its `EquipSlot` names.
/// TODO(X42): replaced by what the actor has equipped from its inventory.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
pub struct WieldedItems(pub Vec<String>);

impl Aspect for WieldedItems {
    fn name(&self) -> &str {
        "WieldedItems"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ACTOR_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<WieldedItems> = RemoveComponent::<WieldedItems>::new();
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
