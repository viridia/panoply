//! How values are laid out in stack slots: a struct's fields one after another, each primitive,
//! entity and string one slot, and anything else one opaque slot referring to it.

use bevy::{
    ecs::entity::Entity,
    reflect::{PartialReflect, ReflectRef, TypeInfo},
};

use crate::{sema::types::ExprType, vm::VMError};

/// What one slot holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Leaf {
    Bool,
    I32,
    I64,
    F32,
    F64,
    Entity,
    Str,
    /// A value scripts cannot see into, such as an enum or a list: a reference to it.
    Opaque,
}

impl Leaf {
    fn of(info: &'static TypeInfo) -> Option<Leaf> {
        Some(if info.is::<bool>() {
            Leaf::Bool
        } else if info.is::<i32>() {
            Leaf::I32
        } else if info.is::<i64>() {
            Leaf::I64
        } else if info.is::<f32>() {
            Leaf::F32
        } else if info.is::<f64>() {
            Leaf::F64
        } else if info.is::<Entity>() {
            Leaf::Entity
        } else if info.is::<String>() {
            Leaf::Str
        } else {
            return None;
        })
    }
}

/// A type's slots.
#[derive(Debug)]
pub(crate) enum Layout {
    Leaf(Leaf),
    /// A struct's, tuple struct's or tuple's fields in order, each at its offset.
    Fields {
        slots: usize,
        fields: Vec<(usize, Layout)>,
    },
    /// `len` elements, one after another.
    Array {
        element: Box<Layout>,
        len: usize,
    },
}

impl Layout {
    /// The layout of a reflected type. A field whose type is not known is opaque.
    pub(crate) fn of(info: &'static TypeInfo) -> Layout {
        if let Some(leaf) = Leaf::of(info) {
            return Layout::Leaf(leaf);
        }
        let field_infos: Vec<Option<&'static TypeInfo>> = match info {
            TypeInfo::Struct(info) => info.iter().map(|f| f.type_info()).collect(),
            TypeInfo::TupleStruct(info) => info.iter().map(|f| f.type_info()).collect(),
            TypeInfo::Tuple(info) => info.iter().map(|f| f.type_info()).collect(),
            TypeInfo::Array(info) => {
                let element = info
                    .item_info()
                    .map_or(Layout::Leaf(Leaf::Opaque), Layout::of);
                return Layout::Array {
                    element: Box::new(element),
                    len: info.capacity(),
                };
            }
            _ => return Layout::Leaf(Leaf::Opaque),
        };
        let mut slots = 0;
        let fields = field_infos
            .into_iter()
            .map(|info| {
                let layout = info.map_or(Layout::Leaf(Leaf::Opaque), Layout::of);
                let offset = slots;
                slots += layout.slots();
                (offset, layout)
            })
            .collect();
        Layout::Fields { slots, fields }
    }

    pub(crate) fn slots(&self) -> usize {
        match self {
            Layout::Leaf(_) => 1,
            Layout::Fields { slots, .. } => *slots,
            Layout::Array { element, len } => element.slots() * len,
        }
    }

    /// The offset and slot count of field `index`.
    pub(crate) fn field(&self, index: usize) -> Option<(usize, usize)> {
        let Layout::Fields { fields, .. } = self else {
            return None;
        };
        fields
            .get(index)
            .map(|(offset, layout)| (*offset, layout.slots()))
    }
}

/// How many slots a value of type `typ` takes, or `None` for a type with no value on the stack
/// (a function, or one not inferred).
pub(crate) fn slot_count(typ: &ExprType) -> Option<usize> {
    match typ {
        ExprType::Void => Some(0),
        ExprType::Boolean
        | ExprType::I32
        | ExprType::I64
        | ExprType::F32
        | ExprType::F64
        | ExprType::String
        | ExprType::Entity => Some(1),
        ExprType::Array(element, len) => Some(slot_count(element)? * len),
        ExprType::Reflected(info) => Some(Layout::of(info).slots()),
        ExprType::None | ExprType::Infer(_) | ExprType::Function(_) => None,
    }
}

/// Where [`flatten`] puts what it reads.
pub(crate) trait SlotSink<'w> {
    fn leaf(&mut self, leaf: Leaf, value: &'w dyn PartialReflect) -> Result<(), VMError>;
}

/// Reads the slots `start..end` of `value`, laid out as `layout`, into `sink`, skipping the
/// fields outside them.
pub(crate) fn flatten<'w>(
    value: &'w dyn PartialReflect,
    layout: &Layout,
    start: usize,
    end: usize,
    sink: &mut impl SlotSink<'w>,
) -> Result<(), VMError> {
    let mismatch = || VMError::Unsupported("a value unlike its type's layout");
    match layout {
        Layout::Leaf(leaf) => sink.leaf(*leaf, value),
        Layout::Fields { fields, .. } => {
            let reflected = value.reflect_ref();
            for (index, (offset, field_layout)) in fields.iter().enumerate() {
                let field_end = offset + field_layout.slots();
                if field_end <= start || *offset >= end {
                    continue;
                }
                let field = match &reflected {
                    ReflectRef::Struct(s) => s.field_at(index),
                    ReflectRef::TupleStruct(s) => s.field(index),
                    ReflectRef::Tuple(t) => t.field(index),
                    _ => None,
                }
                .ok_or_else(mismatch)?;
                flatten(
                    field,
                    field_layout,
                    start.saturating_sub(*offset),
                    (end - offset).min(field_layout.slots()),
                    sink,
                )?;
            }
            Ok(())
        }
        Layout::Array { element, len } => {
            let ReflectRef::Array(array) = value.reflect_ref() else {
                return Err(mismatch());
            };
            let size = element.slots();
            for index in 0..*len {
                let offset = index * size;
                if offset + size <= start || offset >= end {
                    continue;
                }
                let item = array.get(index).ok_or_else(mismatch)?;
                flatten(
                    item,
                    element,
                    start.saturating_sub(offset),
                    (end - offset).min(size),
                    sink,
                )?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        math::{Quat, Vec3},
        reflect::Typed,
        transform::components::Transform,
    };

    use super::*;

    #[test]
    fn glam_and_bevy_types_flatten() {
        assert_eq!(Layout::of(Vec3::type_info()).slots(), 3);
        assert_eq!(Layout::of(Quat::type_info()).slots(), 4);
        let transform = Layout::of(Transform::type_info());
        assert_eq!(transform.slots(), 10);
        assert_eq!(transform.field(1), Some((3, 4)));
        assert_eq!(transform.field(2), Some((7, 3)));
        assert_eq!(Layout::of(<[f32; 4]>::type_info()).slots(), 4);
    }
}
