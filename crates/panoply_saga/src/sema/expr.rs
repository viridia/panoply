use core::mem::needs_drop;

use super::types::{ExprType, TypeRef};
use crate::syntax::{
    location::TokenLocation,
    oper::{BinaryOp, UnaryOp},
};

/// Content of an Expression node.
#[derive(Debug)]
pub(crate) enum ExprKind<'e> {
    /// Expression that represents a bare semicolon.
    Empty,
    ConstInteger(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(&'e str),
    ScriptFunctionRef(usize),
    HostFunctionRef(usize),
    HostMethodRef(&'e Expr<'e>, usize),
    /// A reference to a function parameter.
    ParamRef(usize),
    /// A reference to a local variable.
    LocalRef(usize),
    /// A reference to a host global.
    GlobalRef(usize),
    /// A field of a reflected struct, by index.
    Field(&'e Expr<'e>, usize),
    /// A property of a host type, by its index in the host's members.
    NativeProp(&'e Expr<'e>, usize),
    Call(&'e Expr<'e>, &'e [&'e Expr<'e>]),
    BinaryExpr {
        op: BinaryOp,
        lhs: &'e Expr<'e>,
        rhs: &'e Expr<'e>,
    },
    UnaryExpr {
        op: UnaryOp,
        arg: &'e Expr<'e>,
    },
    Assign {
        lhs: &'e Expr<'e>,
        rhs: &'e Expr<'e>,
    },
    AssignOp {
        op: BinaryOp,
        lhs: &'e Expr<'e>,
        rhs: &'e Expr<'e>,
    },
    Cast(&'e Expr<'e>),
    Block(&'e [&'e Expr<'e>], Option<&'e Expr<'e>>),
    If {
        condition: &'e Expr<'e>,
        then_branch: &'e Expr<'e>,
        else_branch: Option<&'e Expr<'e>>,
    },
}

/// Expression node. Allocated in an arena that never runs destructors, so it owns nothing: its
/// type is a handle into the compilation's type table.
#[derive(Debug)]
pub(crate) struct Expr<'e> {
    pub(crate) location: TokenLocation,
    pub(crate) kind: ExprKind<'e>,
    pub(crate) typ: TypeRef,
}

const _: () = assert!(!needs_drop::<Expr<'static>>());

/// A function's typed tree and frame.
#[derive(Debug)]
pub(crate) struct FunctionBody<'ex> {
    pub(crate) body: Option<&'ex Expr<'ex>>,
    pub(crate) num_params: usize,
    pub(crate) locals: Vec<ExprType>,
}
