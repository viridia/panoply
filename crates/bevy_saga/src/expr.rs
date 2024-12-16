use wasmtime::Func;

use crate::{decl, location::TokenLocation, oper::BinaryOp, types::Type};
use core::fmt::Display;

/// Content of an Expression node.
#[derive(Debug)]
pub(crate) enum ExprKind {
    /// Expression that represents a bare semicolon.
    Empty,
    ConstInteger(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(decl::Symbol),
    DeclRef(decl::DeclId),
    Call(Box<Expr>, Vec<Expr>),
    BinaryExpr {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Cast(Box<Expr>),
    Block(Vec<Expr>, Option<Box<Expr>>),
}

/// Expression node.
#[derive(Debug)]
pub(crate) struct Expr {
    pub(crate) location: TokenLocation,
    pub(crate) kind: ExprKind,
    pub(crate) typ: Type,
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ExprKind::Empty => write!(f, ";"),
            ExprKind::ConstInteger(value) => value.fmt(f),
            ExprKind::ConstFloat(value) => {
                let mut str_val = value.to_string();
                if !str_val.contains(".") {
                    str_val.push_str(".0");
                }
                str_val.fmt(f)
            }
            ExprKind::ConstBool(value) => value.fmt(f),
            ExprKind::ConstString(symbol) => write!(f, "String({})", symbol.0),
            ExprKind::DeclRef(symbol) => write!(f, "Ident({})", symbol.0),
            ExprKind::BinaryExpr {
                op,
                ref lhs,
                ref rhs,
            } => {
                // TODO: Parens if necessary.
                lhs.fmt(f)?;
                match op {
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
            ExprKind::Cast(ref arg) => {
                arg.fmt(f)?;
                write!(f, " as ")?;
                self.typ.fmt(f)
            }
            ExprKind::Call(ref func, ref args) => {
                func.fmt(f)?;
                write!(f, "(")?;
                for arg in args {
                    arg.fmt(f)?;
                    write!(f, ", ")?;
                }
                write!(f, ")")
            }
            ExprKind::Block(ref stmts, ref result) => {
                write!(f, "{{")?;
                for stmt in stmts {
                    stmt.fmt(f)?;
                }
                if let Some(result) = result {
                    write!(f, "; ")?;
                    result.fmt(f)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Expr {
    pub fn new(location: TokenLocation, value: ExprKind) -> Self {
        Self {
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
