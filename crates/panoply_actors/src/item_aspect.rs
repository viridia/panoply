use bevy::prelude::*;
use panoply_exemplar::{annotations::AspectIcon, *};
use serde::{Deserialize, Serialize};

/// The item's model: a `file#Scene` path to a named scene of a props glTF.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/gem.png"))]
pub struct ItemModel(pub String);

impl NativeAspect for ItemModel {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Item
    }
}

/// Where the item is held: the name of the bone its model attaches to (R14).
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/backpack.png"))]
pub struct EquipSlot(pub String);

impl NativeAspect for EquipSlot {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Item
    }
}
