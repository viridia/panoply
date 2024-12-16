use core::result;
use std::sync::Arc;

use crate::{
    ast::{ASTNode, NodeKind},
    compiler::CompilationError,
    decl::{self, Decl},
    expr::{Expr, ExprKind},
    types::{FunctionParam, FunctionType, Type},
};

use super::{
    assign_types::assign_types, resolve_types::resolve_types, type_inference::TypeInference,
};

pub(crate) fn build_module_decls<'ast>(
    symbols: &decl::SymbolTable,
    scope: &mut decl::Scope,
    decls_table: &mut decl::DeclsTable,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    if let NodeKind::Program(decls) = &ast.kind {
        for decl in *decls {
            match &decl.kind {
                NodeKind::Decl(d) => match d {
                    crate::ast::DeclKind::Function {
                        name,
                        body,
                        visibility,
                        ..
                    } => {
                        // Multiple declarations of the same function are not allowed.
                        if scope.contains(*name) {
                            let name_str = symbols.resolve(*name);
                            return Err(CompilationError::FunctionRedefinition(
                                decl.location,
                                name_str,
                            ));
                        }

                        let f = decl::Decl {
                            location: decl.location,
                            visibility: *visibility,
                            name: *name,
                            kind: decl::DeclKind::Function {
                                typ: Arc::new(FunctionType::default()),
                                body: Expr::new(body.location, ExprKind::Empty),
                            },
                        };

                        let id = decls_table.insert(f);
                        scope.insert(*name, id);
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
    symbols: &decl::SymbolTable,
    scope: &mut decl::Scope,
    decls_table: &mut decl::DeclsTable,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    // Resolve types for all declarations.
    if let NodeKind::Program(decls) = &ast.kind {
        for decl_ast in *decls {
            match &decl_ast.kind {
                NodeKind::Decl(d) => match d {
                    crate::ast::DeclKind::Function {
                        name,
                        params,
                        ret: ret_ast,
                        ..
                    } => {
                        let decl_id = scope.get(*name).unwrap();
                        let ret_type = match ret_ast.kind {
                            NodeKind::Empty => Type::Void,
                            _ => resolve_types(symbols, scope, decls_table, ret_ast)?,
                        };

                        let mut params_mapped: Vec<FunctionParam> =
                            Vec::with_capacity(params.len());
                        for p in params.iter() {
                            let typ = resolve_types(symbols, scope, decls_table, p.typ)?;
                            let decl = decls_table.insert(Decl {
                                location: p.location,
                                visibility: decl::DeclVisibility::Private,
                                name: p.name,
                                kind: decl::DeclKind::Param(typ.clone()),
                            });
                            params_mapped.push(FunctionParam {
                                name: p.name,
                                decl,
                                typ,
                            });
                        }

                        let fty = FunctionType {
                            params: params_mapped,
                            ret: ret_type,
                        };

                        let decl = decls_table.get_mut(decl_id);
                        let decl::DeclKind::Function { ref mut typ, .. } = decl.kind else {
                            unreachable!()
                        };
                        *typ = Arc::new(fty);
                    }
                    crate::ast::DeclKind::Let { name, typ, value } => todo!(),
                    crate::ast::DeclKind::Const { name, typ, value } => todo!(),
                    crate::ast::DeclKind::Struct { name, fields } => todo!(),
                    crate::ast::DeclKind::TypeAlias { name, typ } => todo!(),
                },
                _ => panic!("Invalid AST node for declaration: {:?}", decl_ast.kind),
            }
        }

        // Build function body expressions.
        for decl_ast in *decls {
            if let NodeKind::Decl(crate::ast::DeclKind::Function {
                name,
                body: body_ast,
                ..
            }) = &decl_ast.kind
            {
                let decl_id = scope.get(*name).unwrap();
                let mut inference: TypeInference = Default::default();
                let mut body_expr =
                    build_exprs(body_ast, symbols, scope, decls_table, &mut inference)?;
                let decl = decls_table.get_mut(decl_id);
                let decl::DeclKind::Function { ref typ, .. } = decl.kind else {
                    unreachable!()
                };
                inference.add_constraint(
                    typ.ret.clone(),
                    body_expr.typ.clone(),
                    body_expr.location,
                );
                inference.solve_constraints()?;
                assign_types(&mut body_expr, decls_table, &inference)?;
                let decl = decls_table.get_mut(decl_id);
                let decl::DeclKind::Function { ref mut body, .. } = decl.kind else {
                    unreachable!()
                };
                *body = body_expr;
            }
        }

        Ok(())
    } else {
        panic!("Invalid AST node for compilation unit: {:?}", ast.kind);
    }
}

pub(crate) fn build_exprs<'a>(
    ast: &'a ASTNode<'a>,
    symbols: &decl::SymbolTable,
    scope: &decl::Scope,
    decls_table: &mut decl::DeclsTable,
    inference: &mut TypeInference,
) -> Result<Expr, CompilationError> {
    match &ast.kind {
        NodeKind::LitInt(value, suffix) => {
            let typ = match suffix {
                &crate::ast::IntegerSuffix::Unsized => {
                    let ty = inference.fresh_typevar();
                    if *value > i32::MAX as i64 || *value < i32::MIN as i64 {
                        inference.add_constraint(ty.clone(), Type::I64, ast.location);
                    } else {
                        inference.add_constraint(ty.clone(), Type::I32, ast.location);
                    }
                    ty
                }
                crate::ast::IntegerSuffix::I32 => Type::I32,
                crate::ast::IntegerSuffix::I64 => Type::I64,
            };
            Ok(Expr::new(ast.location, ExprKind::ConstInteger(*value)).with_type(typ))
        }

        NodeKind::LitFloat(value, suffix) => {
            // let value = value.parse::<f64>().unwrap();
            let typ = match suffix {
                crate::ast::FloatSuffix::F32 => Type::F32,
                crate::ast::FloatSuffix::F64 => Type::F64,
                _ => unreachable!(),
            };
            Ok(Expr::new(ast.location, ExprKind::ConstFloat(*value)).with_type(typ))
        }

        NodeKind::LitString(value) => {
            Ok(Expr::new(ast.location, ExprKind::ConstString(*value)).with_type(Type::String))
        }

        NodeKind::LitBool(value) => {
            Ok(Expr::new(ast.location, ExprKind::ConstBool(*value)).with_type(Type::Boolean))
        }

        NodeKind::Ident(symbol) => match scope.get(*symbol) {
            Some(sym) => {
                let decl = decls_table.get(sym);
                match &decl.kind {
                    // decl::DeclKind::Function { .. } => {
                    //     Expr::new(ast.location, ExprKind::DeclRef(sym)).with_type(Type::Function)
                    // }
                    decl::DeclKind::Const(typ, _) => {
                        Ok(Expr::new(ast.location, ExprKind::DeclRef(sym)).with_type(typ.clone()))
                    }
                    decl::DeclKind::Let(typ, _) => {
                        Ok(Expr::new(ast.location, ExprKind::DeclRef(sym)).with_type(typ.clone()))
                    }
                    // Struct (constructor)
                    // Enum (constructor)
                    _ => todo!(),
                }
            }
            None => todo!(),
        },

        NodeKind::BinaryExpr { op, lhs, rhs } => {
            let lhs_expr = build_exprs(lhs, symbols, scope, decls_table, inference)?;
            let rhs_expr = build_exprs(rhs, symbols, scope, decls_table, inference)?;
            let ty = match op {
                crate::oper::BinaryOp::Add
                | crate::oper::BinaryOp::Sub
                | crate::oper::BinaryOp::Mul
                | crate::oper::BinaryOp::Div
                | crate::oper::BinaryOp::Mod
                | crate::oper::BinaryOp::BitAnd
                | crate::oper::BinaryOp::BitOr
                | crate::oper::BinaryOp::BitXor => {
                    let ty = inference.fresh_typevar();
                    // Both sides must be the same, which is also the result type.
                    inference.add_constraint(ty.clone(), lhs_expr.typ.clone(), lhs_expr.location);
                    inference.add_constraint(ty.clone(), rhs_expr.typ.clone(), rhs_expr.location);
                    ty
                }

                crate::oper::BinaryOp::LogAnd | crate::oper::BinaryOp::LogOr => {
                    // Both sides must be boolean.
                    inference.add_constraint(
                        Type::Boolean,
                        lhs_expr.typ.clone(),
                        lhs_expr.location,
                    );
                    inference.add_constraint(
                        Type::Boolean,
                        rhs_expr.typ.clone(),
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
                    let ty = inference.fresh_typevar();
                    // Both sides must be the same, but the result type is Boolean.
                    inference.add_constraint(ty.clone(), lhs_expr.typ.clone(), lhs_expr.location);
                    inference.add_constraint(ty.clone(), rhs_expr.typ.clone(), rhs_expr.location);
                    Type::Boolean
                }
            };

            Ok(Expr::new(
                ast.location,
                ExprKind::BinaryExpr {
                    op: *op,
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(rhs_expr),
                },
            )
            .with_type(ty))
        }

        NodeKind::Empty => Ok(Expr::new(ast.location, ExprKind::Empty)),
        NodeKind::Block(stmts, result) => {
            let mut stmt_exprs = Vec::new();
            for stmt in *stmts {
                let stmt_expr = build_exprs(stmt, symbols, scope, decls_table, inference)?;
                stmt_exprs.push(stmt_expr);
            }

            let result_expr = match result {
                Some(result) => Some(Box::new(build_exprs(
                    result,
                    symbols,
                    scope,
                    decls_table,
                    inference,
                )?)),
                None => None,
            };
            let result_type = result_expr
                .as_ref()
                .map(|expr| expr.typ.clone())
                .unwrap_or(Type::None);
            let location = result_expr
                .as_ref()
                .map(|expr| expr.location)
                .unwrap_or(ast.location);
            Ok(
                Expr::new(location, ExprKind::Block(stmt_exprs, result_expr))
                    .with_type(result_type),
            )
        }

        NodeKind::Cast { arg, typ } => {
            let mut infer = TypeInference::default();
            let to_typ = resolve_types(symbols, scope, decls_table, typ)?;
            let mut arg_expr = build_exprs(arg, symbols, scope, decls_table, &mut infer)?;
            infer.solve_constraints()?;

            if to_typ == arg_expr.typ {
                Ok(arg_expr)
            } else if to_typ.is_number() && arg_expr.typ.is_number() {
                assign_types(&mut arg_expr, decls_table, &infer)?;
                Ok(Expr::new(ast.location, ExprKind::Cast(Box::new(arg_expr))).with_type(to_typ))
            } else {
                Err(CompilationError::InvalidCast(
                    ast.location,
                    to_typ.clone(),
                    arg_expr.typ.clone(),
                ))
            }
        }

        _ => {
            panic!("Invalid AST node for expression: {:?}", ast.kind);
        }
    }
}
