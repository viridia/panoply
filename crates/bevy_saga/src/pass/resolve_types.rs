use crate::{
    ast::{self, ASTNode, NodeKind, TypeKind},
    decl::Scope,
    Type,
};

pub(crate) fn resolve_types<'s, 'a>(scope: &'s Scope<'s>, ast: &'a ASTNode<'a>) -> Type {
    match ast.kind {
        NodeKind::Type(ref typ) => match typ {
            TypeKind::Tuple(_members) => todo!(),
            TypeKind::Array(_member) => todo!(),
            TypeKind::Function { params, ret } => todo!(),
            TypeKind::Boolean => Type::Boolean,
            TypeKind::I32 => Type::I32,
            TypeKind::I64 => Type::I64,
            TypeKind::F32 => Type::F32,
            TypeKind::F64 => Type::F64,
            TypeKind::String => Type::String,
        },
        NodeKind::Empty => Type::None,
        _ => {
            panic!("Invalid type expression: {:?}", ast.kind)
        }
    }
}
