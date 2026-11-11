use core::fmt::Display;
use std::sync::Arc;

use bevy::{ecs::entity::Entity, reflect::TypeInfo};
use smol_str::SmolStr;

use super::decl::FunctionParam;

/// Represents the type of an expression.
#[derive(Debug, Default, Clone)]
pub enum ExprType {
    /// No type specified
    #[default]
    None,
    /// Type to be inferred
    Infer(TypeVarId),
    /// No value
    Void,
    Boolean,
    I32,
    I64,
    F32,
    F64,
    String,
    Entity,
    Function(Arc<FunctionType>),
    Reflected(&'static TypeInfo),
}

impl ExprType {
    pub fn from_type_info(type_info: &'static TypeInfo) -> Self {
        if type_info.is::<bool>() {
            ExprType::Boolean
        } else if type_info.is::<i32>() {
            ExprType::I32
        } else if type_info.is::<i64>() {
            ExprType::I64
        } else if type_info.is::<f32>() {
            ExprType::F32
        } else if type_info.is::<f64>() {
            ExprType::F64
        } else if type_info.is::<Entity>() {
            ExprType::Entity
        } else if type_info.is::<String>() {
            ExprType::String
        } else {
            ExprType::Reflected(type_info)
        }
    }

    /// Returns true if the type is void.
    pub fn is_void(&self) -> bool {
        matches!(self, ExprType::Void)
    }

    /// Returns true if the type is an integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, ExprType::I32 | ExprType::I64)
    }

    /// Returns true if the type is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, ExprType::F32 | ExprType::F64)
    }

    /// Returns true if the type is a number.
    pub fn is_number(&self) -> bool {
        self.is_integer() || self.is_float()
    }
}

impl PartialEq for ExprType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Infer(l0), Self::Infer(r0)) => l0 == r0,
            (Self::Function(l0), Self::Function(r0)) => l0 == r0,
            (Self::Reflected(l0), Self::Reflected(r0)) => l0.type_id() == r0.type_id(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Display for ExprType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExprType::None => write!(f, "None"),
            ExprType::Infer(id) => write!(f, "Infer({})", id.0),
            ExprType::Void => write!(f, "void"),
            ExprType::Boolean => write!(f, "bool"),
            ExprType::I32 => write!(f, "i32"),
            ExprType::I64 => write!(f, "i64"),
            ExprType::F32 => write!(f, "f32"),
            ExprType::F64 => write!(f, "f64"),
            ExprType::String => write!(f, "String"),
            ExprType::Entity => write!(f, "Entity"),
            ExprType::Reflected(type_info) => write!(f, "{}", type_info.type_path()),
            ExprType::Function(ftype) => {
                write!(f, "fn(")?;
                for (i, param) in ftype.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    param.typ.fmt(f)?;
                }
                write!(f, ")")?;
                if !ftype.ret.is_void() {
                    write!(f, " -> {}", ftype.ret)?
                }
                Ok(())
            }
        }
    }
}

/// Type data for a function
#[derive(Debug, Default, PartialEq, Clone)]
pub struct FunctionType {
    pub params: Vec<FunctionParam>,
    pub ret: ExprType,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct TypeVarId(pub(crate) usize);

/// A parameter of a host function, as registered.
#[derive(Debug, PartialEq)]
pub struct HostParam {
    pub name: SmolStr,
    pub ty: ExprType,
}

/// The type of an expression in the typed tree: a handle into the compilation's [`TypeTable`],
/// so that the arena-allocated tree owns no types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeRef(u32);

/// The types of one compilation's typed trees, outside the arena.
#[derive(Debug, Default)]
pub(crate) struct TypeTable(Vec<ExprType>);

impl TypeTable {
    pub(crate) fn add(&mut self, typ: ExprType) -> TypeRef {
        let index = self.0.len();
        self.0.push(typ);
        TypeRef(index as u32)
    }

    pub(crate) fn get(&self, typ: TypeRef) -> &ExprType {
        &self.0[typ.0 as usize]
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Replaces the types from `start` on, as when one function's inference variables are
    /// substituted.
    pub(crate) fn map_from(&mut self, start: usize, mut f: impl FnMut(&ExprType) -> ExprType) {
        for typ in self.0[start..].iter_mut() {
            *typ = f(typ);
        }
    }
}
