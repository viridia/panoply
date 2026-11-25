//! What slots refer to during an evaluation, and how Rust values go in and out of slots.

use bevy::{
    ecs::entity::Entity,
    math::{Quat, Vec2, Vec3, Vec4},
    reflect::PartialReflect,
    transform::components::Transform,
};

use super::{StackValue, VMError};

/// A string a slot names: borrowed for the evaluation, or owned by it.
enum StrEntry<'w> {
    Borrowed(&'w str),
    Owned(usize),
}

/// What the evaluation's slots refer to: its strings and reflected values. A slot holds an index
/// into one of these, so slots own nothing; all of it is freed when the evaluation ends.
#[derive(Default)]
pub struct SlotStore<'w> {
    strings: Vec<StrEntry<'w>>,
    owned: Vec<String>,
    world_refs: Vec<&'w dyn PartialReflect>,
    heap_refs: Vec<Box<dyn PartialReflect>>,
}

impl<'w> SlotStore<'w> {
    pub(crate) fn clear(&mut self) {
        self.strings.clear();
        self.owned.clear();
        self.world_refs.clear();
        self.heap_refs.clear();
    }

    fn add_string(&mut self, entry: StrEntry<'w>) -> Result<StackValue, VMError> {
        let index = u32::try_from(self.strings.len())
            .map_err(|_| VMError::Unsupported("more than 4294967295 strings"))?;
        self.strings.push(entry);
        Ok(StackValue::Str(index))
    }

    /// A slot naming `s`, a string that outlives the evaluation.
    pub fn borrowed_str(&mut self, s: &'w str) -> Result<StackValue, VMError> {
        self.add_string(StrEntry::Borrowed(s))
    }

    /// A slot naming `s`, kept until the evaluation ends.
    pub fn owned_str(&mut self, s: String) -> Result<StackValue, VMError> {
        let index = self.owned.len();
        self.owned.push(s);
        self.add_string(StrEntry::Owned(index))
    }

    /// The string a slot's index names.
    pub fn str(&self, index: u32) -> Option<&str> {
        match self.strings.get(index as usize)? {
            StrEntry::Borrowed(s) => Some(s),
            StrEntry::Owned(index) => self.owned.get(*index).map(String::as_str),
        }
    }

    /// A slot referring to a reflected value in the world.
    pub fn world_ref(&mut self, value: &'w dyn PartialReflect) -> StackValue {
        let index = self.world_refs.len();
        self.world_refs.push(value);
        StackValue::WorldRef(index)
    }

    /// A slot holding a reflected temporary, freed when the evaluation ends.
    pub fn heap_ref<T: PartialReflect>(&mut self, value: T) -> Result<StackValue, VMError> {
        let index = u16::try_from(self.heap_refs.len())
            .map_err(|_| VMError::Unsupported("more than 65535 temporaries"))?;
        self.heap_refs.push(Box::new(value).into_partial_reflect());
        Ok(StackValue::HeapRef(index))
    }

    /// The reflected value an opaque slot refers to.
    pub fn reflected(&self, value: &StackValue) -> Option<&dyn PartialReflect> {
        match value {
            StackValue::WorldRef(index) => self.world_refs.get(*index).copied(),
            StackValue::HeapRef(index) => self.heap_refs.get(*index as usize).map(|r| r.as_ref()),
            _ => None,
        }
    }

    /// The type of what a slot holds, for error messages.
    pub(crate) fn type_path(&self, value: &StackValue) -> &'static str {
        match value {
            StackValue::Void => "void",
            StackValue::Bool(_) => "bool",
            StackValue::I32(_) => "i32",
            StackValue::I64(_) => "i64",
            StackValue::F32(_) => "f32",
            StackValue::F64(_) => "f64",
            StackValue::Entity(_) => "Entity",
            StackValue::Str(_) => "String",
            StackValue::WorldRef(_) | StackValue::HeapRef(_) => self
                .reflected(value)
                .and_then(|r| r.get_represented_type_info())
                .map_or("none", |info| info.type_path()),
        }
    }
}

/// A Rust value read from slots: a native's argument, or an evaluation's result. A value read
/// from an opaque or string slot borrows from the evaluation, for `'a`.
pub trait FromSlots<'a>: Sized {
    /// How many slots the value takes.
    const SLOTS: usize;

    /// Reads the value from exactly [`Self::SLOTS`] slots.
    fn from_slots(slots: &[StackValue], store: &'a SlotStore<'_>) -> Result<Self, VMError>;
}

/// A Rust value written to slots: a native's result.
pub trait ToSlots {
    fn to_slots(&self, out: &mut Vec<StackValue>, store: &mut SlotStore<'_>)
    -> Result<(), VMError>;
}

fn mismatch(store: &SlotStore, found: &StackValue, expected: &'static str) -> VMError {
    VMError::MismatchedTypes2(store.type_path(found), expected)
}

/// The one slot of a one-slot value.
fn single(slots: &[StackValue]) -> Result<&StackValue, VMError> {
    match slots {
        [slot] => Ok(slot),
        _ => Err(VMError::StackUnderflow),
    }
}

macro_rules! primitive_slots {
    ($($ty:ty => $variant:ident),*) => {$(
        impl FromSlots<'_> for $ty {
            const SLOTS: usize = 1;

            fn from_slots(slots: &[StackValue], store: &SlotStore<'_>) -> Result<Self, VMError> {
                match single(slots)? {
                    StackValue::$variant(value) => Ok(*value),
                    other => Err(mismatch(store, other, stringify!($ty))),
                }
            }
        }

        impl ToSlots for $ty {
            fn to_slots(
                &self,
                out: &mut Vec<StackValue>,
                _store: &mut SlotStore<'_>,
            ) -> Result<(), VMError> {
                out.push(StackValue::$variant(*self));
                Ok(())
            }
        }
    )*};
}

primitive_slots!(bool => Bool, i32 => I32, i64 => I64, f32 => F32, f64 => F64, Entity => Entity);

impl ToSlots for StackValue {
    fn to_slots(
        &self,
        out: &mut Vec<StackValue>,
        _store: &mut SlotStore<'_>,
    ) -> Result<(), VMError> {
        out.push(*self);
        Ok(())
    }
}

impl<'a> FromSlots<'a> for &'a str {
    const SLOTS: usize = 1;

    fn from_slots(slots: &[StackValue], store: &'a SlotStore<'_>) -> Result<Self, VMError> {
        match single(slots)? {
            StackValue::Str(index) => store.str(*index).ok_or(VMError::InvalidString(*index)),
            other => Err(mismatch(store, other, "String")),
        }
    }
}

impl FromSlots<'_> for String {
    const SLOTS: usize = 1;

    fn from_slots(slots: &[StackValue], store: &SlotStore<'_>) -> Result<Self, VMError> {
        <&str>::from_slots(slots, store).map(str::to_owned)
    }
}

impl ToSlots for &str {
    fn to_slots(
        &self,
        out: &mut Vec<StackValue>,
        store: &mut SlotStore<'_>,
    ) -> Result<(), VMError> {
        out.push(store.owned_str(self.to_string())?);
        Ok(())
    }
}

impl ToSlots for String {
    fn to_slots(
        &self,
        out: &mut Vec<StackValue>,
        store: &mut SlotStore<'_>,
    ) -> Result<(), VMError> {
        self.as_str().to_slots(out, store)
    }
}

/// An opaque value, borrowed from the evaluation.
impl<'a> FromSlots<'a> for &'a dyn PartialReflect {
    const SLOTS: usize = 1;

    fn from_slots(slots: &[StackValue], store: &'a SlotStore<'_>) -> Result<Self, VMError> {
        let slot = single(slots)?;
        store
            .reflected(slot)
            .ok_or_else(|| mismatch(store, slot, "a reflected value"))
    }
}

/// A vector of `f32` fields, one slot each, built by its `new`.
macro_rules! float_struct_slots {
    ($($ty:ty { $($field:ident),* }),*) => {$(
        impl FromSlots<'_> for $ty {
            const SLOTS: usize = [$(stringify!($field)),*].len();

            fn from_slots(slots: &[StackValue], store: &SlotStore<'_>) -> Result<Self, VMError> {
                let [$($field),*] = slots else {
                    return Err(VMError::StackUnderflow);
                };
                Ok(<$ty>::new($(f32::from_slots(core::slice::from_ref($field), store)?),*))
            }
        }

        impl ToSlots for $ty {
            fn to_slots(
                &self,
                out: &mut Vec<StackValue>,
                _store: &mut SlotStore<'_>,
            ) -> Result<(), VMError> {
                out.extend([$(StackValue::F32(self.$field)),*]);
                Ok(())
            }
        }
    )*};
}

float_struct_slots!(Vec2 { x, y }, Vec3 { x, y, z }, Vec4 { x, y, z, w });

impl FromSlots<'_> for Quat {
    const SLOTS: usize = 4;

    fn from_slots(slots: &[StackValue], store: &SlotStore<'_>) -> Result<Self, VMError> {
        Ok(Quat::from_vec4(Vec4::from_slots(slots, store)?))
    }
}

impl ToSlots for Quat {
    fn to_slots(
        &self,
        out: &mut Vec<StackValue>,
        store: &mut SlotStore<'_>,
    ) -> Result<(), VMError> {
        Vec4::from(*self).to_slots(out, store)
    }
}

impl FromSlots<'_> for Transform {
    const SLOTS: usize = 10;

    fn from_slots(slots: &[StackValue], store: &SlotStore<'_>) -> Result<Self, VMError> {
        if slots.len() != Self::SLOTS {
            return Err(VMError::StackUnderflow);
        }
        Ok(Transform {
            translation: Vec3::from_slots(&slots[0..3], store)?,
            rotation: Quat::from_slots(&slots[3..7], store)?,
            scale: Vec3::from_slots(&slots[7..10], store)?,
        })
    }
}

impl ToSlots for Transform {
    fn to_slots(
        &self,
        out: &mut Vec<StackValue>,
        store: &mut SlotStore<'_>,
    ) -> Result<(), VMError> {
        self.translation.to_slots(out, store)?;
        self.rotation.to_slots(out, store)?;
        self.scale.to_slots(out, store)
    }
}

impl<'a, T: FromSlots<'a>, const N: usize> FromSlots<'a> for [T; N] {
    const SLOTS: usize = T::SLOTS * N;

    fn from_slots(slots: &[StackValue], store: &'a SlotStore<'_>) -> Result<Self, VMError> {
        if slots.len() != Self::SLOTS || T::SLOTS == 0 {
            return Err(VMError::StackUnderflow);
        }
        let items = slots
            .chunks_exact(T::SLOTS)
            .map(|chunk| T::from_slots(chunk, store))
            .collect::<Result<Vec<T>, VMError>>()?;
        items.try_into().map_err(|_| VMError::StackUnderflow)
    }
}

/// A tuple reads its members from consecutive slots, as a native memo's arguments.
macro_rules! tuple_from_slots {
    ($($member:ident),+) => {
        impl<'a, $($member: FromSlots<'a>),+> FromSlots<'a> for ($($member,)+) {
            const SLOTS: usize = 0 $(+ $member::SLOTS)+;

            // The last member's end is never read.
            #[allow(unused_assignments)]
            fn from_slots(slots: &[StackValue], store: &'a SlotStore<'_>) -> Result<Self, VMError> {
                if slots.len() != Self::SLOTS {
                    return Err(VMError::StackUnderflow);
                }
                let mut start = 0;
                Ok(($({
                    let end = start + $member::SLOTS;
                    let member = $member::from_slots(&slots[start..end], store)?;
                    start = end;
                    member
                },)+))
            }
        }
    };
}

tuple_from_slots!(A);
tuple_from_slots!(A, B);
tuple_from_slots!(A, B, C);
tuple_from_slots!(A, B, C, D);

impl<T: ToSlots, const N: usize> ToSlots for [T; N] {
    fn to_slots(
        &self,
        out: &mut Vec<StackValue>,
        store: &mut SlotStore<'_>,
    ) -> Result<(), VMError> {
        for item in self {
            item.to_slots(out, store)?;
        }
        Ok(())
    }
}
