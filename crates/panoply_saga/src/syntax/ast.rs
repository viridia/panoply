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
    /// A tuple's or tuple struct's field: `v.0`.
    FieldIndex(&'a ASTNode<'a>, usize),
    /// `[a, b, c]`.
    ArrayLit(&'a [&'a ASTNode<'a>]),
    /// `a[i]`.
    Index(&'a ASTNode<'a>, &'a ASTNode<'a>),
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
    /// `[T; N]`, or `[T]` with no length.
    ArrayType(&'a ASTNode<'a>, Option<usize>),
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
    /// `#[attr] aspect Name for Kind, Kind { field: Type = default, state field: Type }`, with
    /// its interactions among the fields.
    Aspect {
        attrs: &'a [&'a ASTAttribute<'a>],
        name: &'a str,
        kinds: &'a [&'a str],
        fields: &'a [&'a ASTAspectField<'a>],
        interactions: &'a [&'a ASTInteraction<'a>],
    },
}

/// `interaction name { field: value, ... }` in an aspect: a struct literal of the interaction
/// type the host registers.
#[derive(Debug)]
pub(crate) struct ASTInteraction<'a> {
    pub(crate) location: TokenLocation,
    pub(crate) name: &'a str,
    pub(crate) fields: &'a [&'a ASTInteractionField<'a>],
}

/// One field of an interaction: its name and the expression or block giving its value.
#[derive(Debug)]
pub(crate) struct ASTInteractionField<'a> {
    pub(crate) location: TokenLocation,
    pub(crate) name: &'a str,
    pub(crate) value: &'a ASTNode<'a>,
}

/// `#[name]` or `#[name(arg, ...)]`, before a declaration.
#[derive(Debug)]
pub(crate) struct ASTAttribute<'a> {
    pub(crate) location: TokenLocation,
    pub(crate) name: &'a str,
    pub(crate) args: &'a [&'a ASTNode<'a>],
}

/// A field of an aspect declaration.
#[derive(Debug)]
pub(crate) struct ASTAspectField<'a> {
    pub(crate) location: TokenLocation,
    pub(crate) attrs: &'a [&'a ASTAttribute<'a>],
    pub(crate) name: &'a str,
    /// Whether it is `state`, which play changes, rather than config.
    pub(crate) is_state: bool,
    pub(crate) typ: &'a ASTNode<'a>,
    pub(crate) default: Option<&'a ASTNode<'a>>,
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
const _: () = assert!(!needs_drop::<ASTAttribute<'static>>());
const _: () = assert!(!needs_drop::<ASTAspectField<'static>>());
const _: () = assert!(!needs_drop::<ASTInteraction<'static>>());
const _: () = assert!(!needs_drop::<ASTInteractionField<'static>>());
