use core::mem::needs_drop;

use super::{
    location::TokenLocation,
    oper::{BinaryOp, UnaryOp},
};
use crate::sema::decl::DeclVisibility;

/// AST node. Allocated in an arena that never runs destructors, so it owns nothing.
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

/// Content of an AST node.
#[derive(Debug)]
pub(crate) enum NodeKind<'a> {
    Module(&'a [&'a ASTNode<'a>]),
    Empty,
    /// `import a::b::{c, d};` or `import a::b::c;`: the path, then the names imported from it,
    /// which are empty when the path's last segment is the one name imported.
    // TODO(PAN71): resolve the path and bind the names.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "imports are parsed but not yet resolved")
    )]
    Import(&'a [&'a str], &'a [&'a str]),
    Decl(&'a ASTDecl<'a>),
    Block(&'a [&'a ASTNode<'a>], Option<&'a ASTNode<'a>>),
    LitBool(bool),
    LitInt(i64, IntegerSuffix),
    LitFloat(f64, FloatSuffix),
    LitString(&'a str),
    Ident(&'a str),
    QName(&'a [&'a ASTNode<'a>]),
    Call(&'a ASTNode<'a>, &'a [&'a ASTNode<'a>]),
    FieldName(&'a ASTNode<'a>, &'a str),
    // TODO(X7): the base and index, once tuple fields compile.
    FieldIndex,
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
    Assign {
        lhs: &'a ASTNode<'a>,
        rhs: &'a ASTNode<'a>,
    },
    AssignOp {
        op: BinaryOp,
        lhs: &'a ASTNode<'a>,
        rhs: &'a ASTNode<'a>,
    },
    // TODO(X7): the element type, once fixed-size arrays compile.
    ArrayType,
    If {
        condition: &'a ASTNode<'a>,
        then_block: &'a ASTNode<'a>,
        else_block: Option<&'a ASTNode<'a>>,
    },
}

/// Content of an AST declaration.
#[derive(Debug)]
pub(crate) enum ASTDecl<'a> {
    Function {
        name: &'a str,
        visibility: DeclVisibility,
        params: &'a [&'a ASTFunctionParam<'a>],
        ret: Option<&'a ASTNode<'a>>,
        body: &'a ASTNode<'a>,
    },
    Let {
        name: &'a str,
        visibility: DeclVisibility,
        is_const: bool,
        typ: Option<&'a ASTNode<'a>>,
        value: Option<&'a ASTNode<'a>>,
    },
    // TODO(X87): the name and fields, once script-declared structs compile.
    Struct,
}

/// AST for a function parameter declaration
#[derive(Debug)]
pub(crate) struct ASTFunctionParam<'a> {
    pub(crate) location: TokenLocation,
    pub(crate) name: &'a str,
    pub(crate) typ: &'a ASTNode<'a>,
}

const _: () = assert!(!needs_drop::<ASTNode<'static>>());
const _: () = assert!(!needs_drop::<ASTDecl<'static>>());
const _: () = assert!(!needs_drop::<ASTFunctionParam<'static>>());
