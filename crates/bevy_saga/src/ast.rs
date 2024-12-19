use crate::{
    decl::{DeclVisibility, Symbol},
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
    pub fn new(location: impl Into<TokenLocation>, kind: NodeKind<'a>) -> Self {
        Self {
            location: location.into(),
            kind,
        }
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
    Program(&'a [&'a ASTNode<'a>]),
    Empty,
    Decl(&'a DeclKind<'a>),
    Block(&'a [&'a ASTNode<'a>], Option<&'a ASTNode<'a>>),
    LitBool(bool),
    LitInt(i64, IntegerSuffix),
    LitFloat(f64, FloatSuffix),
    LitString(Symbol),
    Ident(Symbol),
    QName(&'a [&'a ASTNode<'a>]),
    Call(&'a ASTNode<'a>, &'a [&'a ASTNode<'a>]),
    Field(&'a ASTNode<'a>, Symbol),
    Cast {
        arg: &'a ASTNode<'a>,
        typ: &'a ASTNode<'a>,
    },
    UnaryExpr {
        op: UnaryOp,
        arg: &'a ASTNode<'a>,
    },
    BinaryExpr {
        op: BinaryOp,
        lhs: &'a ASTNode<'a>,
        rhs: &'a ASTNode<'a>,
    },
    Type(TypeKind<'a>),
}

// Content of an AST declaration.
#[derive(Debug)]
pub(crate) enum DeclKind<'a> {
    Function {
        name: Symbol,
        visibility: DeclVisibility,
        params: &'a [&'a FunctionParam<'a>],
        ret: Option<&'a ASTNode<'a>>,
        body: Option<&'a ASTNode<'a>>,
        is_native: bool,
    },
    Let {
        name: Symbol,
        is_const: bool,
        typ: Option<&'a ASTNode<'a>>,
        value: Option<&'a ASTNode<'a>>,
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
    pub(crate) location: TokenLocation,
    pub(crate) name: Symbol,
    pub(crate) typ: &'a ASTNode<'a>,
    // pub(crate) value: &'a ASTNode<'a>,
}

/// Represents an AST for a type expression
#[derive(Debug)]
pub enum TypeKind<'a> {
    Tuple(&'a [&'a ASTNode<'a>]),
    Array(&'a ASTNode<'a>),
    Function {
        params: &'a [&'a ASTNode<'a>],
        ret: &'a ASTNode<'a>,
    },
    // TODO: Struct, Record, Option, Enum
}
