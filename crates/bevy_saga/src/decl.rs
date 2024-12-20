use core::cell::RefCell;
use std::sync::Arc;

use bevy::utils::HashMap;

use crate::{
    expr::Expr,
    location::TokenLocation,
    types::{FunctionType, StructType, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub(crate) usize);

struct SymbolTableInner {
    symbols: HashMap<String, Symbol>,
    strings: Vec<String>,
}

pub(crate) struct InternedSymbols(RefCell<SymbolTableInner>);

impl InternedSymbols {
    pub fn new() -> Self {
        Self(RefCell::new(SymbolTableInner {
            symbols: HashMap::new(),
            strings: Vec::new(),
        }))
    }

    pub fn intern(&self, name: &str) -> Symbol {
        let mut inner = self.0.borrow_mut();
        match inner.symbols.get(name) {
            Some(symbol) => *symbol,
            None => {
                let id = inner.strings.len();
                inner.strings.push(name.to_string());
                let symbol = Symbol(id);
                inner.symbols.insert(name.to_string(), symbol);
                symbol
            }
        }
    }

    pub fn resolve(&self, symbol: Symbol) -> String {
        let inner = self.0.borrow();
        inner.strings[symbol.0].clone()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DeclVisibility {
    Public,
    #[default]
    Private,
}

#[derive(Debug)]
pub enum Decl {
    Local(usize),
    Global(usize),
    Param(Type, usize),
    Function(usize),
    Struct(usize),
    // Enum(usize),
    TypeAlias(Type),
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub location: TokenLocation,
    pub name: Symbol,
    pub visibility: DeclVisibility,
    pub typ: Arc<FunctionType>,
    pub body: Expr,
    pub locals: Vec<LocalDecl>,
    pub is_native: bool,
    /// Index of this function in the module's functions table.
    pub function_index: usize,
}

/// Declaration for a local variable or constant
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct LocalDecl {
    pub location: TokenLocation,
    pub visibility: DeclVisibility,
    pub name: Symbol,
    pub typ: Type,
    pub index: usize,
    /// Index of this param in the function's local variables. This takes into account multi-value
    /// params.
    pub local_index: usize,
    pub is_const: bool,
}

/// Declaration for a local variable or constant
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct GlobalDecl {
    pub location: TokenLocation,
    pub visibility: DeclVisibility,
    pub name: Symbol,
    pub typ: Type,
    pub index: usize,
    pub is_const: bool,
}

/// Declaration for a function parameter
#[derive(Debug, PartialEq, Clone)]
pub struct ParamDecl {
    pub location: TokenLocation,
    pub name: Symbol,
    pub typ: Type,

    /// Index of this param in the params table.
    pub index: usize,

    /// Index of this param in the function's local variables. This takes into account multi-value
    /// params.
    pub local_index: usize,
}

#[derive(Debug)]
#[allow(unused)]
pub struct StructDecl {
    pub location: TokenLocation,
    pub name: Symbol,
    pub visibility: DeclVisibility,
    pub typ: Arc<StructType>,
    pub index: usize,
}

/// Declaration for a function parameter
#[derive(Debug, PartialEq, Clone)]
pub struct FieldDecl {
    pub location: TokenLocation,
    pub name: Symbol,
    pub typ: Type,
    pub index: usize,
}

#[derive(Debug)]
pub struct Decls {
    pub structs: Vec<StructDecl>,
    pub globals: Vec<GlobalDecl>,
    pub functions: Vec<FunctionDecl>,
}

impl Decls {
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub fn add_function(&mut self, decl: FunctionDecl) -> usize {
        let index = self.functions.len();
        self.functions.push(decl);
        index
    }
}

pub(crate) struct Scope<'a> {
    pub(crate) parent: Option<&'a Scope<'a>>,
    pub(crate) decls: HashMap<Symbol, Decl>,
}

impl<'a> Scope<'a> {
    pub fn new(parent: Option<&'a Scope>) -> Self {
        Self {
            parent,
            decls: HashMap::new(),
        }
    }

    pub fn lookup(&self, name: Symbol) -> Option<&Decl> {
        if let Some(decl) = self.decls.get(&name) {
            return Some(decl);
        }

        if let Some(parent) = self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    pub fn get(&self, name: Symbol) -> Option<&Decl> {
        self.decls.get(&name)
    }

    pub fn insert(&mut self, name: Symbol, decl: Decl) {
        self.decls.insert(name, decl);
    }

    pub fn contains(&self, symbol: Symbol) -> bool {
        self.decls.contains_key(&symbol)
    }
}
