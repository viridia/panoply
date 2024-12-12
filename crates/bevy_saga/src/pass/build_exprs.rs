use std::result;

use crate::{
    ast::{ASTNode, NodeKind},
    compiler::{CompilationError, CompilationUnit},
    decl,
    expr::{Expr, ExprKind},
    types::Type,
};

use super::{
    assign_types::assign_types, resolve_types::resolve_types, type_inference::TypeInference,
};

pub(crate) fn build_module_decls<'ast>(
    unit: &mut CompilationUnit,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    if let NodeKind::Program(decls) = &ast.kind {
        for decl in *decls {
            match &decl.kind {
                NodeKind::Decl(d) => match d {
                    crate::ast::DeclKind::Function {
                        name, params, body, ..
                    } => {
                        // Multiple declarations of the same function are not allowed.
                        if unit.root_scope.contains(*name) {
                            let name_str = unit.symbols.resolve(*name);
                            return Err(CompilationError::FunctionRedefinition(
                                decl.location,
                                name_str,
                            ));
                        }

                        let f = decl::Decl {
                            loc: decl.location,
                            name: *name,
                            kind: decl::DeclKind::Function {
                                name: *name,
                                params: params
                                    .iter()
                                    .map(|p| decl::Decl {
                                        loc: p.location,
                                        name: p.name,
                                        kind: decl::DeclKind::Param(Type::None),
                                    })
                                    .collect(),
                                ret: Type::None,
                                body: Expr::new(body.location, ExprKind::Empty),
                            },
                        };

                        unit.root_scope.insert(f);
                    }
                    crate::ast::DeclKind::Let { name, typ, value } => todo!(),
                    crate::ast::DeclKind::Const { name, typ, value } => todo!(),
                    crate::ast::DeclKind::Struct { name, fields } => todo!(),
                    crate::ast::DeclKind::TypeAlias { name, typ } => todo!(),
                },
                _ => panic!("Invalid AST node for declaration: {:?}", decl.kind),
            }
        }

        Ok(())
    } else {
        panic!("Invalid AST node for compilation unit: {:?}", ast.kind);
    }
}

pub(crate) fn build_module_exprs<'ast>(
    unit: &mut CompilationUnit,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    if let NodeKind::Program(decls) = &ast.kind {
        for decl_ast in *decls {
            match &decl_ast.kind {
                NodeKind::Decl(d) => match d {
                    crate::ast::DeclKind::Function {
                        name,
                        body: body_ast,
                        ret: ret_ast,
                        ..
                    } => {
                        let decl = unit.root_scope.get(*name).unwrap();
                        let decl::DeclKind::Function { ref params, .. } = decl.kind else {
                            unreachable!()
                        };
                        let mut inference: TypeInference = Default::default();
                        let mut body_expr = build_exprs(unit, body_ast, &mut inference);
                        let ret_type = resolve_types(&unit.root_scope, ret_ast);
                        inference.add_constraint(
                            body_expr.typ.clone(),
                            ret_type.clone(),
                            body_expr.location,
                        );
                        inference.solve_constraints();
                        assign_types(&mut body_expr, &inference)?;

                        let decl = unit.root_scope.get_mut(*name).unwrap();
                        let decl::DeclKind::Function {
                            ref mut body,
                            ref mut ret,
                            ..
                        } = decl.kind
                        else {
                            unreachable!()
                        };
                        *body = body_expr;
                        *ret = ret_type;
                    }
                    crate::ast::DeclKind::Let { name, typ, value } => todo!(),
                    crate::ast::DeclKind::Const { name, typ, value } => todo!(),
                    crate::ast::DeclKind::Struct { name, fields } => todo!(),
                    crate::ast::DeclKind::TypeAlias { name, typ } => todo!(),
                },
                _ => panic!("Invalid AST node for declaration: {:?}", decl_ast.kind),
            }
        }

        Ok(())
    } else {
        panic!("Invalid AST node for compilation unit: {:?}", ast.kind);
    }
}

pub(crate) fn build_exprs<'a>(
    unit: &'a mut CompilationUnit,
    ast: &'a ASTNode<'a>,
    inference: &mut TypeInference,
) -> Expr {
    match &ast.kind {
        NodeKind::ConstInteger(value, suffix) => {
            let value = value.parse::<i64>().unwrap();
            let typ = match suffix {
                &crate::ast::IntegerSuffix::Unsized => {
                    let ty = unit.fresh_typevar();
                    if value > i32::MAX as i64 || value < i32::MIN as i64 {
                        inference.add_constraint(ty.clone(), Type::I64, ast.location);
                    } else {
                        inference.add_constraint(ty.clone(), Type::I32, ast.location);
                    }
                    ty
                }
                crate::ast::IntegerSuffix::I32 => Type::I32,
                crate::ast::IntegerSuffix::I64 => Type::I64,
            };
            Expr::new(ast.location, ExprKind::ConstInteger(value)).with_type(typ)
        }

        NodeKind::ConstFloat(value, suffix) => {
            let value = value.parse::<f64>().unwrap();
            let typ = match suffix {
                crate::ast::FloatSuffix::F32 => Type::F32,
                crate::ast::FloatSuffix::F64 => Type::F64,
                _ => unreachable!(),
            };
            Expr::new(ast.location, ExprKind::ConstFloat(value)).with_type(typ)
        }

        NodeKind::String(_) => todo!(),
        NodeKind::Ident(_) => todo!(),

        NodeKind::BinaryExpr { op, lhs, rhs } => {
            let lhs_expr = build_exprs(unit, lhs, inference);
            let rhs_expr = build_exprs(unit, rhs, inference);
            let ty = match op {
                crate::oper::BinaryOp::MemberAccess => todo!(),

                crate::oper::BinaryOp::Add
                | crate::oper::BinaryOp::Sub
                | crate::oper::BinaryOp::Mul
                | crate::oper::BinaryOp::Div
                | crate::oper::BinaryOp::Mod
                | crate::oper::BinaryOp::BitAnd
                | crate::oper::BinaryOp::BitOr
                | crate::oper::BinaryOp::BitXor => {
                    let ty = unit.fresh_typevar();
                    // Both sides must be the same, which is also the result type.
                    inference.add_constraint(ty.clone(), lhs_expr.typ.clone(), lhs_expr.location);
                    inference.add_constraint(ty.clone(), rhs_expr.typ.clone(), rhs_expr.location);
                    ty
                }

                crate::oper::BinaryOp::LogAnd | crate::oper::BinaryOp::LogOr => {
                    // Both sides must be boolean.
                    inference.add_constraint(
                        lhs_expr.typ.clone(),
                        Type::Boolean,
                        lhs_expr.location,
                    );
                    inference.add_constraint(
                        rhs_expr.typ.clone(),
                        Type::Boolean,
                        rhs_expr.location,
                    );
                    Type::Boolean
                }

                crate::oper::BinaryOp::Shl | crate::oper::BinaryOp::Shr => todo!(),

                crate::oper::BinaryOp::Eq | crate::oper::BinaryOp::Ne => todo!(),

                crate::oper::BinaryOp::Lt
                | crate::oper::BinaryOp::Le
                | crate::oper::BinaryOp::Gt
                | crate::oper::BinaryOp::Ge => {
                    // Both sides must be the same.
                    inference.add_constraint(
                        lhs_expr.typ.clone(),
                        rhs_expr.typ.clone(),
                        ast.location,
                    );
                    Type::Boolean
                }
            };

            Expr::new(
                ast.location,
                ExprKind::BinaryExpr {
                    op: *op,
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(rhs_expr),
                },
            )
            .with_type(ty)
        }

        NodeKind::Empty => Expr::new(ast.location, ExprKind::Empty),
        NodeKind::Block(stmts, result) => {
            let mut stmt_exprs = Vec::new();
            for stmt in *stmts {
                let stmt_expr = build_exprs(unit, stmt, inference);
                stmt_exprs.push(stmt_expr);
            }

            let result_expr = result.map(|result| Box::new(build_exprs(unit, result, inference)));
            let result_type = result_expr
                .as_ref()
                .map(|expr| expr.typ.clone())
                .unwrap_or(Type::None);
            Expr::new(ast.location, ExprKind::Block(stmt_exprs, result_expr)).with_type(result_type)
        }

        _ => {
            panic!("Invalid AST node for expression: {:?}", ast.kind);
        }
    }
}
