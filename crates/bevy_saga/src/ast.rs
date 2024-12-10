use crate::{expr::Symbol, location::TokenLocation, oper::BinaryOp};

/// AST node.
#[derive(Debug)]
pub(crate) struct ASTNode<'a> {
    pub(crate) location: TokenLocation,
    pub(crate) kind: NodeKind<'a>,
}

impl<'a> ASTNode<'a> {
    pub fn new(location: TokenLocation, kind: NodeKind<'a>) -> Self {
        Self { location, kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum IntegerSuffix {
    Unsized,
    I32,
    I64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FloatSuffix {
    F32,
    F64,
}

// Content of an AST node.
#[derive(Debug)]
pub(crate) enum NodeKind<'a> {
    ConstInteger(&'a str, IntegerSuffix),
    ConstFloat(&'a str, FloatSuffix),
    String(Symbol),
    Ident(Symbol),
    BinaryExpr {
        op: BinaryOp,
        lhs: &'a ASTNode<'a>,
        rhs: &'a ASTNode<'a>,
    },
}
