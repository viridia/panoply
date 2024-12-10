use crate::{
    expr::Symbol,
    location::TokenLocation,
    oper::{BinaryOp, UnaryOp},
};

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
    Unit(&'a [&'a ASTNode<'a>]),
    Decl(&'a DeclKind<'a>),
    ConstInteger(&'a str, IntegerSuffix),
    ConstFloat(&'a str, FloatSuffix),
    String(Symbol),
    Ident(Symbol),
    UnaryExpr {
        op: UnaryOp,
        arg: &'a ASTNode<'a>,
    },
    BinaryExpr {
        op: BinaryOp,
        lhs: &'a ASTNode<'a>,
        rhs: &'a ASTNode<'a>,
    },
}

// Content of an AST declaration.
#[derive(Debug)]
pub(crate) enum DeclKind<'a> {
    Function {
        name: Symbol,
        params: &'a [&'a FunctionParam<'a>],
        ret: &'a ASTNode<'a>,
        body: &'a ASTNode<'a>,
    },
    Let {
        name: Symbol,
        typ: &'a ASTNode<'a>,
        value: &'a ASTNode<'a>,
    },
    Const {
        name: Symbol,
        typ: &'a ASTNode<'a>,
        value: &'a ASTNode<'a>,
    },
    Struct {
        name: Symbol,
        fields: &'a [&'a ASTNode<'a>],
    },
    TypeAlias {
        name: Symbol,
        typ: &'a ASTNode<'a>,
    },
}

/// AST for a function parameter
#[derive(Debug)]
pub(crate) struct FunctionParam<'a> {
    name: Symbol,
    typ: &'a ASTNode<'a>,
    value: &'a ASTNode<'a>,
}
