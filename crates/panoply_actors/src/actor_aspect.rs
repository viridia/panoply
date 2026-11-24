use std::collections::BTreeMap;

use bevy::{annotations::DisplayName, prelude::*};
use panoply_exemplar::{
    annotations::{AspectIcon, Precision, ReflectPropertyMapKeys},
    *,
};
use serde::{Deserialize, Serialize};

use panoply_core::HexColor;

/// The species' skeleton, which the engine animates, and the file holding its clips.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/bone.png"))]
#[serde(default)]
pub struct Armature {
    /// The skeleton's root node in the `Skin`'s scene; by default the `Skin` file's one armature.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[reflect(@DisplayName("Node"))]
    pub node: Option<String>,
    /// The character glTF holding the species' clips, often shared by several species; by
    /// default the `Skin`'s own file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[reflect(@DisplayName("Clips"))]
    pub clips: Option<ClipsFile>,
}

/// A character glTF holding animation clips, by its path.
#[derive(Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Default)]
pub struct ClipsFile(pub String);

impl NativeAspect for Armature {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// Renames the clips the engine asks for to the species' own, where they differ: humanoids share
/// one clips file whose gaits are named by gender (`"Walk": "WalkMale"`).
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/film.png"))]
pub struct ClipNames(pub BTreeMap<String, String>);

impl NativeAspect for ClipNames {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// The actor's collision capsule (R4), in meters: its whole height, and its radius.
#[derive(Component, Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/cylinder.png"))]
pub struct CollisionSize {
    #[reflect(@DisplayName("Height"))]
    pub height: f32,
    #[reflect(@DisplayName("Radius"))]
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

impl NativeAspect for CollisionSize {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// Draws the actor's body this many times its modelled size, and scales its `CollisionSize` to
/// match: one species' model serves a large or small individual.
#[derive(Component, Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/scale-3d.png"))]
pub struct ActorScale(#[reflect(@Precision(2))] pub f32);

impl Default for ActorScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl NativeAspect for ActorScale {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// The actor's body: a `file#Scene` path to a named scene of a character glTF, which holds the
/// body mesh and the skeleton it is bound to.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/shirt.png"))]
pub struct Skin(pub String);

impl NativeAspect for Skin {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// The actor's colors, by material name: its color slots are the materials of its skin and its
/// features marked as slots (`actor_slots`). A feature takes its colors from these too.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize, PropertyMapKeys)]
#[reflect(@AspectIcon("editor/icons/lucide/palette.png"))]
pub struct Colors(pub BTreeMap<String, HexColor>);

impl NativeAspect for Colors {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// The actor's features, by scene name, each shown while `true`: its feature slots are its skin
/// file's feature scenes (`actor_slots`). A key that names no feature is kept but unused.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize, PropertyMapKeys)]
#[reflect(@AspectIcon("editor/icons/lucide/sparkles.png"))]
pub struct Features(pub BTreeMap<String, bool>);

impl NativeAspect for Features {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}

/// Items the actor holds, as item exemplar paths, each shown on the bone its `EquipSlot` names.
/// TODO(X42): replaced by what the actor has equipped from its inventory.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/sword.png"))]
pub struct WieldedItems(pub Vec<String>);

impl NativeAspect for WieldedItems {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }
}
