use std::any::TypeId;

use bevy::platform::collections::HashMap;
use smol_str::SmolStr;

use super::types::ExprType;
use crate::syntax::location::TokenLocation;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DeclVisibility {
    Public,
    #[default]
    Private,
}

#[derive(Debug)]
pub struct Decl {
    pub location: TokenLocation,
    pub visibility: DeclVisibility,
    pub typ: ExprType,
    pub kind: DeclKind,
}

#[derive(Debug)]
pub enum DeclKind {
    Global {
        is_const: bool,
        /// Index of this variable in the host or module's variables table.
        index: usize,
    },
    Local {
        is_const: bool,
        /// Index of this variable in the local variables table.
        index: usize,
    },
    Param {
        /// Index of this variable in the current function's parameter list.
        index: usize,
    },
    Function {
        /// Index of this function in the host or module's functions table.
        index: usize,
    },
    NativeType {
        /// Index of this type in the host types table.
        id: TypeId,
    },
    TypeAlias,
}

/// Declaration for a function parameter
#[derive(Debug, PartialEq, Clone)]
pub struct FunctionParam {
    pub location: TokenLocation,
    pub name: SmolStr,
    pub typ: ExprType,

    /// Index of this param in the params table.
    pub index: usize,
}

pub type DeclTable = HashMap<SmolStr, Decl>;

/// What kind of scope this is.
#[derive(Debug, Clone, Copy)]
pub enum ScopeType {
    Host,
    Module,
    Param,
    Local,
}

#[derive(Debug)]
pub(crate) struct Scope<'parent, 'decls> {
    pub(crate) parent: Option<&'parent Scope<'parent, 'parent>>,
    pub(crate) decls: &'decls DeclTable,
    pub(crate) scope_type: ScopeType,
}

impl Scope<'_, '_> {
    pub fn lookup(&self, name: &str) -> Option<(ScopeType, &Decl)> {
        if let Some(decl) = self.decls.get(name) {
            return Some((self.scope_type, decl));
        }

        if let Some(parent) = self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }
}
