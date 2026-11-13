use bevy::prelude::*;
use panoply_exemplar::{annotations::AspectIcon, *};
use serde::{Deserialize, Serialize};

use super::ITEM_TYPE;

/// The item's model: a `file#Scene` path to a named scene of a props glTF.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/gem.png"))]
pub struct ItemModel(pub String);

impl Aspect for ItemModel {
    fn name(&self) -> &str {
        "ItemModel"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ITEM_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<ItemModel> = RemoveComponent::<ItemModel>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// Where the item is held: the name of the bone its model attaches to (R14).
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/backpack.png"))]
pub struct EquipSlot(pub String);

impl Aspect for EquipSlot {
    fn name(&self) -> &str {
        "EquipSlot"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == ITEM_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<EquipSlot> = RemoveComponent::<EquipSlot>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
