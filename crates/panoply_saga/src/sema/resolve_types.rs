use super::{
    decl::{DeclKind, Scope},
    types::ExprType,
};
use crate::{
    compiler::CompilationError,
    syntax::ast::{ASTNode, NodeKind},
};

/// Convert AST types to type expressions.
pub(crate) fn resolve_types(scope: &Scope, ast: &ASTNode) -> Result<ExprType, CompilationError> {
    match &ast.kind {
        NodeKind::Ident(ident) => {
            let Some((_, decl)) = scope.lookup(ident) else {
                return Err(CompilationError::UnknownType(ast.location, (*ident).into()));
            };
            match &decl.kind {
                DeclKind::TypeAlias | DeclKind::NativeType { .. } => Ok(decl.typ.clone()),
                _ => Err(CompilationError::UnknownType(ast.location, (*ident).into())),
            }
        }
        // TODO(X7): fixed-size arrays, laid out in slots.
        NodeKind::ArrayType => Err(CompilationError::Unsupported(ast.location, "array types")),
        // TODO(PAN71): types named through an imported module.
        NodeKind::QName(_) => Err(CompilationError::Unsupported(
            ast.location,
            "qualified type names",
        )),
        _ => Err(CompilationError::Expected(ast.location, "type".into())),
    }
}
