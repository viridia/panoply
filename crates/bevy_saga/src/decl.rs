use core::cell::RefCell;
use std::sync::Arc;

use bevy::utils::HashMap;

use crate::{
    expr::Expr,
    location::TokenLocation,
    types::{FunctionType, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclId(pub(crate) usize);

struct SymbolTableInner {
    symbols: HashMap<String, Symbol>,
    strings: Vec<String>,
}

pub(crate) struct SymbolTable(RefCell<SymbolTableInner>);

impl SymbolTable {
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

pub(crate) struct DeclsTable {
    decls: Vec<Decl>,
}

impl DeclsTable {
    pub fn new() -> Self {
        Self { decls: Vec::new() }
    }

    pub fn insert(&mut self, decl: Decl) -> DeclId {
        let id = DeclId(self.decls.len());
        self.decls.push(decl);
        id
    }

    pub fn get(&self, id: DeclId) -> &Decl {
        &self.decls[id.0]
    }

    pub fn get_mut(&mut self, id: DeclId) -> &mut Decl {
        &mut self.decls[id.0]
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DeclVisibility {
    Public,
    #[default]
    Private,
}

#[derive(Debug)]
pub struct Decl {
    #[allow(unused)]
    pub location: TokenLocation,
    pub visibility: DeclVisibility,
    pub name: Symbol,
    pub kind: DeclKind,
}

#[derive(Debug)]
pub enum DeclKind {
    Const(Type, Expr),
    Let(Type, Expr),
    Param(Type),
    Function { typ: Arc<FunctionType>, body: Expr },
    Struct(Type),
    Enum(Type),
    Type(Type),
}

pub(crate) struct Scope<'a> {
    pub(crate) parent: Option<&'a Scope<'a>>,
    pub(crate) decls: HashMap<Symbol, DeclId>,
}

impl<'a> Scope<'a> {
    pub fn new(parent: Option<&'a Scope>) -> Self {
        Self {
            parent,
            decls: HashMap::new(),
        }
    }

    pub fn lookup(&self, name: Symbol) -> Option<DeclId> {
        if let Some(decl) = self.decls.get(&name) {
            return Some(*decl);
        }

        if let Some(parent) = self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    pub fn get(&self, name: Symbol) -> Option<DeclId> {
        self.decls.get(&name).copied()
    }

    pub fn insert(&mut self, name: Symbol, decl: DeclId) {
        self.decls.insert(name, decl);
    }

    pub fn contains(&self, symbol: Symbol) -> bool {
        self.decls.contains_key(&symbol)
    }
}
