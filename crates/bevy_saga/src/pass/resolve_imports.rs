use core::result;
use std::sync::Arc;

use bevy::{render::render_graph::Node, scene::ron::de};

use crate::{
    ast::{ASTNode, NodeKind},
    compiler::CompilationError,
    decl::{self, Decl, ImportDecl, LocalDecl, ParamDecl, Scope},
    expr::{Expr, ExprKind},
    types::{FunctionType, StructType, Type},
    CompilationUnit,
};

use super::{
    assign_types::assign_types, resolve_types::resolve_types, type_inference::TypeInference,
};

pub(crate) fn define_imports<'ast>(
    decls: &mut decl::Decls,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    if let NodeKind::Program(ast_decls) = &ast.kind {
        for ast_decl in *ast_decls {
            if let NodeKind::Import(path_expr, names) = &ast_decl.kind {
                if let NodeKind::LitString(path_sym) = path_expr.kind {
                    decls.imports.push(ImportDecl {
                        location: ast_decl.location,
                        path: path_sym,
                        names: names.to_vec(),
                    });
                }
            }
        }

        Ok(())
    } else {
        panic!("Invalid AST node for compilation unit: {:?}", ast.kind);
    }
}
