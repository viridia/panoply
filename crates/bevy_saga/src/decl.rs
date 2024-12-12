use std::cell::RefCell;

use bevy::utils::HashMap;

use crate::{expr::Expr, location::TokenLocation, types::Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Symbol(pub(crate) usize);

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

pub(crate) struct Decl {
    pub(crate) loc: TokenLocation,
    pub(crate) name: Symbol,
    pub(crate) kind: DeclKind,
}

pub enum DeclKind {
    Const(Type, Expr),
    Let(Type, Expr),
    Param(Type),
    Function {
        name: Symbol,
        params: Vec<Decl>,
        ret: Type,
        body: Expr,
    },
    Struct(Type),
    Enum(Type),
}

pub(crate) struct Scope<'a> {
    pub(crate) parent: Option<&'a Scope<'a>>,
    pub(crate) decls: HashMap<Symbol, Decl>,
}

impl<'a> Scope<'a> {
    pub(crate) fn new(parent: Option<&'a Scope>) -> Self {
        Self {
            parent,
            decls: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, symbol: Symbol) -> Option<&Decl> {
        self.decls.get(&symbol)
    }

    pub(crate) fn get_mut(&mut self, symbol: Symbol) -> Option<&mut Decl> {
        self.decls.get_mut(&symbol)
    }

    pub(crate) fn insert(&mut self, decl: Decl) {
        self.decls.insert(decl.name, decl);
    }

    pub(crate) fn contains(&self, symbol: Symbol) -> bool {
        self.decls.contains_key(&symbol)
    }
}
