use crate::{
    ast::{self, ASTNode, NodeKind, TypeKind},
    decl::{self, Scope},
    CompilationError, Type,
};

/// Convert AST types to type expressions.
pub(crate) fn resolve_types<'s, 'a>(
    symbols: &decl::InternedSymbols,
    scope: &'s Scope<'s>,
    ast: &'a ASTNode<'a>,
) -> Result<Type, CompilationError> {
    match ast.kind {
        NodeKind::Type(ref typ) => match typ {
            TypeKind::Tuple(_members) => todo!(),
            TypeKind::Array(_member) => todo!(),
            TypeKind::Function { params, ret } => todo!(),
        },
        NodeKind::Ident(ident) => {
            let Some(decl) = scope.lookup(ident) else {
                let name = symbols.resolve(ident);
                return Err(CompilationError::UnknownType(ast.location, name));
            };
            match decl {
                decl::Decl::Type(ref typ) => Ok(typ.clone()),
                _ => panic!("Invalid type expression: {:?}", decl),
            }
        }
        NodeKind::Empty => Ok(Type::None),
        _ => {
            panic!("Invalid type expression: {:?}", ast.kind)
        }
    }
}
