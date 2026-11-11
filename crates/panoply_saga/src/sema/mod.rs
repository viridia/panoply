//! AST to typed tree: names, types and checks.

pub(crate) mod build;
pub(crate) mod check;
pub mod decl;
pub(crate) mod expr;
pub(crate) mod infer;
pub(crate) mod resolve_types;
pub mod types;
