use std::{
    cell::RefCell,
    fmt::{write, Display},
};

use bevy::utils::HashMap;

use crate::{location::TokenLocation, oper::BinaryOp, types::Type};

pub(crate) type NodeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub(crate) struct Symbol(pub(crate) usize);

pub(crate) struct SymbolTable(RefCell<HashMap<String, Symbol>>);

impl SymbolTable {
    pub fn new() -> Self {
        Self(RefCell::new(HashMap::new()))
    }

    pub fn intern(&self, name: &str) -> Symbol {
        let mut symbols = self.0.borrow_mut();
        match symbols.get(name) {
            Some(symbol) => *symbol,
            None => {
                let id = symbols.len();
                let symbol = Symbol(id);
                symbols.insert(name.to_string(), symbol);
                symbol
            }
        }
    }

    // pub fn get(&self, name: &str) -> Option<Symbol> {
    //     self.0.get(name).copied()
    // }
}

/// Content of an Expression node.
#[derive(Debug)]
pub(crate) enum ExprKind {
    ConstInteger(i64),
    ConstFloat(f64),
    String(Symbol),
    Ident(Symbol),
    BinaryExpr {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

/// Expression node.
#[derive(Debug)]
pub(crate) struct Expr {
    pub(crate) id: NodeId,
    pub(crate) location: TokenLocation,
    pub(crate) kind: ExprKind,
    pub(crate) typ: Type,
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ExprKind::ConstInteger(value) => value.fmt(f),
            ExprKind::ConstFloat(value) => {
                let mut str_val = value.to_string();
                if !str_val.contains(".") {
                    str_val.push_str(".0");
                }
                str_val.fmt(f)
            }
            ExprKind::String(symbol) => write!(f, "String({})", symbol.0),
            ExprKind::Ident(symbol) => write!(f, "Ident({})", symbol.0),
            ExprKind::BinaryExpr {
                op,
                ref lhs,
                ref rhs,
            } => {
                // TODO: Parens if necessary.
                lhs.fmt(f)?;
                match op {
                    BinaryOp::MemberAccess => write!(f, "."),
                    BinaryOp::Add => write!(f, " + "),
                    BinaryOp::Sub => write!(f, " - "),
                    BinaryOp::Mul => write!(f, " * "),
                    BinaryOp::Div => write!(f, " / "),
                    BinaryOp::Mod => write!(f, " % "),
                    BinaryOp::LogAnd => write!(f, " && "),
                    BinaryOp::LogOr => write!(f, " || "),
                    BinaryOp::BitAnd => write!(f, " & "),
                    BinaryOp::BitOr => write!(f, " | "),
                    BinaryOp::BitXor => write!(f, " ^ "),
                    BinaryOp::Shl => write!(f, " << "),
                    BinaryOp::Shr => write!(f, " >> "),
                    BinaryOp::Eq => write!(f, " == "),
                    BinaryOp::Ne => write!(f, " != "),
                    BinaryOp::Lt => write!(f, " < "),
                    BinaryOp::Le => write!(f, " <= "),
                    BinaryOp::Gt => write!(f, " > "),
                    BinaryOp::Ge => write!(f, " >= "),
                }?;
                rhs.fmt(f)
            }
        }
    }
}

impl Expr {
    pub fn new(id: NodeId, location: TokenLocation, value: ExprKind) -> Self {
        Self {
            id: 0,
            location,
            kind: value,
            typ: Type::None,
        }
    }

    pub fn with_type(mut self, typ: Type) -> Self {
        self.typ = typ;
        self
    }
}
