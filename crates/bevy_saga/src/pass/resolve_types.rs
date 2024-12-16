use crate::{
    ast::{self, ASTNode, NodeKind, TypeKind},
    decl::{self, DeclKind, Scope},
    CompilationError, Type,
};

/// Convert AST types to type expressions.
pub(crate) fn resolve_types<'s, 'a>(
    symbols: &decl::SymbolTable,
    scope: &'s Scope<'s>,
    decls_table: &mut decl::DeclsTable,
    ast: &'a ASTNode<'a>,
) -> Result<Type, CompilationError> {
    match ast.kind {
        NodeKind::Type(ref typ) => match typ {
            TypeKind::Tuple(_members) => todo!(),
            TypeKind::Array(_member) => todo!(),
            TypeKind::Function { params, ret } => todo!(),
            TypeKind::Boolean => Ok(Type::Boolean),
            TypeKind::I32 => Ok(Type::I32),
            TypeKind::I64 => Ok(Type::I64),
            TypeKind::F32 => Ok(Type::F32),
            TypeKind::F64 => Ok(Type::F64),
            TypeKind::String => Ok(Type::String),
        },
        NodeKind::Ident(ident) => {
            let Some(decl_id) = scope.lookup(ident) else {
                let name = symbols.resolve(ident);
                return Err(CompilationError::UnknownType(ast.location, name));
            };
            let decl = decls_table.get(decl_id);
            match decl.kind {
                DeclKind::Type(ref typ) => Ok(typ.clone()),
                _ => panic!("Invalid type expression: {:?}", decl.kind),
            }
        }
        NodeKind::Empty => Ok(Type::None),
        _ => {
            panic!("Invalid type expression: {:?}", ast.kind)
        }
    }
}
