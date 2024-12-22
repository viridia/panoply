use crate::{
    ast::{self, ASTNode, NodeKind},
    decl::{self, Decls, Scope},
    CompilationError, Type,
};

/// Convert AST types to type expressions.
pub(crate) fn resolve_types<'s, 'a>(
    decls: &Decls,
    scope: &'s Scope<'s>,
    ast: &'a ASTNode<'a>,
) -> Result<Type, CompilationError> {
    match ast.kind {
        NodeKind::ArrayType(_member) => todo!(),
        NodeKind::Ident(ident) => {
            let Some(decl) = scope.lookup(ident) else {
                let name = decls.symbols.resolve(ident);
                return Err(CompilationError::UnknownType(ast.location, name));
            };
            match decl {
                decl::Decl::TypeAlias(ref typ) => Ok(typ.clone()),
                decl::Decl::Struct(sindex) => Ok(Type::Struct(decls.structs[*sindex].typ.clone())),
                _ => panic!("Invalid type expression: {:?}", decl),
            }
        }
        NodeKind::Empty => Ok(Type::None),
        _ => {
            panic!("Invalid type expression: {:?}", ast.kind)
        }
    }
}
