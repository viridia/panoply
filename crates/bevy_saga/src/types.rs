use std::{fmt::Display, sync::Arc};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Clone)]
pub enum TypeError {
    #[error("Mismatched types")]
    MismatchedTypes,
    #[error("Recursive type: {0}")]
    RecursiveType(Arc<Type>),
}

/// Represents a SAGA data type.
#[derive(Debug, Default, PartialEq, Clone)]
pub enum Type {
    #[default]
    None, // No type specified
    Infer(TypeVarId), // Type to be inferred
    Boolean,
    IUnsized,
    I32,
    I64,
    F32,
    F64,
    String,
    Tuple(Arc<[Type]>),
    Array(Arc<Type>),
    Error(TypeError),
    // Function {
    //     params: SmallVec<[Param<'a>; 2]>,
    //     ret: &'a Type<'a>,
    // },
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::None => write!(f, "None"),
            Type::Infer(id) => write!(f, "Infer({})", id.0),
            Type::Boolean => write!(f, "bool"),
            Type::IUnsized => write!(f, "{{integer}}"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::String => write!(f, "String"),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    ty.fmt(f)?;
                }
                write!(f, ")")
            }
            Type::Array(ty) => write!(f, "[{}]", ty),
            Type::Error(err) => write!(f, "Error({})", err),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct TypeVarId(pub(crate) usize);

#[derive(Debug, PartialEq)]
pub struct TypeId(usize);

/// Represents a SAGA function parameter.
#[derive(Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeId,
}
