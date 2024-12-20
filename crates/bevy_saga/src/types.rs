use core::fmt::Display;
use std::sync::Arc;

use crate::decl::{FieldDecl, ParamDecl};

/// Represents a SAGA data type.
#[derive(Debug, Default, PartialEq, Clone)]
pub enum Type {
    #[default]
    None, // No type specified
    Infer(TypeVarId), // Type to be inferred
    Void,             // No value
    Boolean,
    IUnsized,
    I32,
    I64,
    F32,
    F64,
    String,
    Tuple(Arc<[Type]>),
    Array(Arc<Type>),
    Function(Arc<FunctionType>),
    Struct(Arc<StructType>),
    TupleStruct(Arc<TupleStructType>),
    // TODO: Struct, Record, Option, Enum
}

/// Type data for a function
#[derive(Debug, Default, PartialEq, Clone)]
pub struct FunctionType {
    pub params: Vec<ParamDecl>,
    pub ret: Type,
}

/// Type data for a struct or record
#[derive(Debug, Default, PartialEq, Clone)]
pub struct StructType {
    pub is_record: bool,
    pub fields: Vec<FieldDecl>,
}

/// Type data for a tuple struct or record
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TupleStructType {
    pub is_record: bool,
    pub fields: Vec<Type>,
}

impl Type {
    /// Returns true if the type is void.
    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    /// Returns true if the type is a tuple.
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::I32 | Type::I64)
    }

    /// Returns true if the type is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    /// Returns true if the type is a number.
    pub fn is_number(&self) -> bool {
        self.is_integer() || self.is_float()
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Type::None => write!(f, "None"),
            Type::Infer(id) => write!(f, "Infer({})", id.0),
            Type::Void => write!(f, "void"),
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
            Type::Function(ftype) => {
                write!(f, "(")?;
                for (i, param) in ftype.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    param.typ.fmt(f)?;
                }
                if !matches!(ftype.ret, Type::None) {
                    write!(f, ") -> {}", ftype.ret)?
                }
                Ok(())
            }
            Type::Struct(stype) => {
                if stype.is_record {
                    write!(f, "record")?;
                } else {
                    write!(f, "struct")?;
                }
                Ok(())
            }
            Type::TupleStruct(stype) => {
                if stype.is_record {
                    write!(f, "tuple_record")?;
                } else {
                    write!(f, "tuple_struct")?;
                }
                Ok(())
            }
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
