use thiserror::Error;

use std::{cell::RefCell, sync::Arc};
// use thiserror::Error;

use bevy::utils::HashMap;

use crate::{
    expr::{Expr, Symbol},
    location::TokenLocation,
    oper::BinaryOp,
    types::TypeVarId,
    Type,
};

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("Expression expected")]
    ExpectExpression(TokenLocation),
    #[error("Mismatched types: {1} {2}")]
    MismatchedTypes(TokenLocation, Type, Type),
    #[error("Recursive type: {1}")]
    RecursiveType(TokenLocation, Type),
    #[error("Invalid type for binary operator {2} to {3}")]
    InvalidBinaryOpType(TokenLocation, BinaryOp, Type, Type),
}

/// Contains all of the information needed to compile a single script file.
pub struct CompilationUnit {
    pub(crate) next_expr_id: usize,
    pub(crate) next_typevar_id: usize,
    pub(crate) symbols: RefCell<Vec<String>>,
    pub(crate) decls: HashMap<Symbol, Decl>,
}

impl Default for CompilationUnit {
    fn default() -> Self {
        Self {
            next_expr_id: 0,
            next_typevar_id: 0,
            symbols: RefCell::new(Vec::new()),
            decls: HashMap::default(),
        }
    }
}

impl CompilationUnit {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn alloc_symbol(&self, value: &str) -> Symbol {
        let mut symbols = self.symbols.borrow_mut();
        let id = Symbol(symbols.len());
        symbols.push(value.to_string());
        id
    }

    pub(crate) fn next_expr_id(&mut self) -> usize {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        id
    }

    pub(crate) fn next_typevar_id(&mut self) -> TypeVarId {
        let id = self.next_typevar_id;
        self.next_typevar_id += 1;
        TypeVarId(id)
    }
}

pub(crate) struct Decl {
    // pub(crate) id: NodeId,
    pub(crate) name: Symbol,
    pub(crate) loc: TokenLocation,
    pub(crate) typ: TypeVarId,
}

pub enum DeclKind {
    Const(Type, Expr),
    Function(Type),
    Struct(Type),
    Enum(Type),
}
