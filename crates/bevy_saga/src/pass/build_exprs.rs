use core::result;
use std::sync::Arc;

use bevy::{render::render_graph::Node, scene::ron::de, utils::tracing::field};

use crate::{
    ast::{ASTNode, NodeKind},
    compiler::CompilationError,
    decl::{self, Decl, LocalDecl, ParamDecl, Scope},
    expr::{Expr, ExprKind},
    types::{FunctionType, StructType, Type},
    CompilationUnit,
};

use super::{
    assign_types::assign_types, resolve_types::resolve_types, type_inference::TypeInference,
};

pub(crate) fn build_module_decls<'ast>(
    scope: &mut decl::Scope,
    decls: &mut decl::Decls,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    if let NodeKind::Program(ast_decls) = &ast.kind {
        for ast_decl in *ast_decls {
            match &ast_decl.kind {
                NodeKind::Decl(d) => match d {
                    crate::ast::ASTDecl::Function {
                        name,
                        visibility,
                        is_native,
                        ..
                    } => {
                        // Multiple declarations of the same function are not allowed.
                        if scope.contains(*name) {
                            let name_str = decls.symbols.resolve(*name);
                            return Err(CompilationError::FunctionRedefinition(
                                ast_decl.location,
                                name_str,
                            ));
                        }

                        let fd = decl::FunctionDecl {
                            location: ast_decl.location,
                            name: *name,
                            visibility: *visibility,
                            typ: Arc::new(FunctionType::default()),
                            body: Expr::new(ast_decl.location, ExprKind::Empty),
                            locals: Vec::new(),
                            is_native: *is_native,
                            function_index: 0,
                        };

                        let findex = decls.add_function(fd);
                        scope.insert(*name, decl::Decl::Function(findex));
                    }

                    crate::ast::ASTDecl::Let {
                        name,
                        visibility,
                        is_const,
                        ..
                    } => {
                        // Multiple declarations of the same function are not allowed.
                        if scope.contains(*name) {
                            let name_str = decls.symbols.resolve(*name);
                            return Err(CompilationError::NameRedefinition(
                                ast_decl.location,
                                name_str,
                            ));
                        }

                        let index = decls.globals.len();
                        let gd = decl::GlobalDecl {
                            location: ast_decl.location,
                            visibility: *visibility,
                            name: *name,
                            typ: Type::None,
                            is_const: *is_const,
                            index,
                        };

                        decls.globals.push(gd);
                        scope.insert(*name, decl::Decl::Global(index));
                    }

                    crate::ast::ASTDecl::Struct {
                        name, visibility, ..
                    } => {
                        // Multiple declarations of the same function are not allowed.
                        if scope.contains(*name) {
                            let name_str = decls.symbols.resolve(*name);
                            return Err(CompilationError::NameRedefinition(
                                ast_decl.location,
                                name_str,
                            ));
                        }

                        let index = decls.structs.len();
                        let sd = decl::StructDecl {
                            location: ast_decl.location,
                            name: *name,
                            visibility: *visibility,
                            typ: Arc::new(StructType::default()),
                            index,
                        };

                        decls.structs.push(sd);
                        scope.insert(*name, decl::Decl::Struct(index));
                    } // crate::ast::ASTDecl::TypeAlias { .. } => todo!(),
                },
                _ => panic!("Invalid AST node for declaration: {:?}", ast_decl.kind),
            }
        }

        // Assign indices to functions. Imported functions first.
        let mut index = 0;
        for fd in decls.functions.iter_mut() {
            if fd.is_native {
                fd.function_index = index;
                index += 1;
            }
        }

        // Then local functions.
        for fd in decls.functions.iter_mut() {
            if !fd.is_native {
                fd.function_index = index;
                index += 1;
            }
        }

        Ok(())
    } else {
        panic!("Invalid AST node for compilation unit: {:?}", ast.kind);
    }
}

pub(crate) fn build_module_exprs<'ast>(
    scope: &mut decl::Scope,
    decls: &mut decl::Decls,
    ast: &'ast ASTNode<'ast>,
) -> Result<(), CompilationError> {
    // Resolve types for all declarations.
    if let NodeKind::Program(ast_decls) = &ast.kind {
        for ast_decl in *ast_decls {
            match &ast_decl.kind {
                NodeKind::Decl(d) => match d {
                    crate::ast::ASTDecl::Function {
                        name,
                        params,
                        ret: ret_ast,
                        ..
                    } => {
                        let decl = scope.get(*name).unwrap();
                        let ret_type = match ret_ast {
                            None => Type::Void,
                            Some(ret_ast) => resolve_types(decls, scope, ret_ast)?,
                        };

                        let mut params_mapped: Vec<ParamDecl> = Vec::with_capacity(params.len());
                        for (i, p) in params.iter().enumerate() {
                            let typ = resolve_types(decls, scope, p.typ)?;
                            params_mapped.push(ParamDecl {
                                location: p.location,
                                name: p.name,
                                typ,
                                index: i,
                                local_index: 0,
                            });
                        }

                        let decl::Decl::Function(findex) = decl else {
                            unreachable!()
                        };
                        decls.functions[*findex].typ = Arc::new(FunctionType {
                            params: params_mapped,
                            ret: ret_type,
                        });
                    }
                    crate::ast::ASTDecl::Let { .. } => todo!(),
                    crate::ast::ASTDecl::Struct {
                        name,
                        is_record,
                        fields,
                        ..
                    } => {
                        let decl = scope.get(*name).unwrap();
                        let decl::Decl::Struct(sindex) = decl else {
                            unreachable!()
                        };

                        let mut stype = StructType {
                            name: *name,
                            is_record: *is_record,
                            fields: Vec::with_capacity(fields.len()),
                        };

                        for (i, f) in fields.iter().enumerate() {
                            let typ = resolve_types(decls, scope, f.typ)?;
                            stype.fields.push(decl::FieldDecl {
                                location: f.location,
                                name: f.name,
                                typ,
                                index: i,
                            });
                        }

                        let sd = &mut decls.structs[*sindex];
                        sd.typ = Arc::new(stype);
                    } // crate::ast::ASTDecl::TypeAlias { .. } => todo!(),
                },
                _ => panic!("Invalid AST node for declaration: {:?}", ast_decl.kind),
            }
        }

        // Build function body expressions.
        for decl_ast in *ast_decls {
            if let NodeKind::Decl(crate::ast::ASTDecl::Function {
                name,
                body: body_ast,
                ..
            }) = &decl_ast.kind
            {
                let decl = scope.get(*name).unwrap();
                let decl::Decl::Function(findex) = decl else {
                    unreachable!()
                };

                let mut inference: TypeInference = Default::default();
                let mut param_scope = Scope::new(Some(scope));
                let function = &mut decls.functions[*findex];
                for param in function.typ.params.iter() {
                    param_scope.insert(
                        param.name,
                        decl::Decl::Param(param.typ.clone(), param.index),
                    );
                }
                let mut local_scope = Scope::new(Some(&param_scope));
                let mut locals_table: Vec<LocalDecl> = Vec::new();
                let mut body_expr = if let Some(body_ast) = body_ast {
                    if function.is_native {
                        return Err(CompilationError::NativeFunctionHasBody(function.location));
                    }
                    build_exprs(
                        body_ast,
                        &mut local_scope,
                        decls,
                        &mut locals_table,
                        &mut inference,
                    )?
                } else {
                    if !function.is_native {
                        return Err(CompilationError::MissingBody(function.location));
                    }
                    Expr::new(decl_ast.location, ExprKind::Empty)
                };
                let function = &mut decls.functions[*findex];
                if body_ast.is_some() {
                    inference.add_constraint(
                        function.typ.ret.clone(),
                        body_expr.typ.clone(),
                        body_expr.location,
                    );
                    inference.solve_constraints()?;
                    assign_types(&mut body_expr, &inference)?;
                    for local in locals_table.iter_mut() {
                        local.typ = inference.substitute(&local.typ);
                    }
                }
                function.body = body_expr;
                function.locals = locals_table;
            }
        }

        Ok(())
    } else {
        panic!("Invalid AST node for compilation unit: {:?}", ast.kind);
    }
}

pub(crate) fn build_exprs<'a>(
    ast: &'a ASTNode<'a>,
    scope: &mut decl::Scope,
    decls: &mut decl::Decls,
    locals: &mut Vec<decl::LocalDecl>,
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
            };
            Ok(Expr::new(ast.location, ExprKind::ConstFloat(*value)).with_type(typ))
        }

        NodeKind::LitString(value) => {
            Ok(Expr::new(ast.location, ExprKind::ConstString(*value)).with_type(Type::String))
        }

        NodeKind::LitBool(value) => {
            Ok(Expr::new(ast.location, ExprKind::ConstBool(*value)).with_type(Type::Boolean))
        }

        NodeKind::Ident(symbol) => match scope.lookup(*symbol) {
            Some(decl) => {
                match &decl {
                    decl::Decl::Function(findex) => {
                        let function = &decls.functions[*findex];
                        // println!("Calling function: {:?}", function.index);
                        Ok(
                            Expr::new(ast.location, ExprKind::FunctionRef(function.function_index))
                                .with_type(Type::Function(function.typ.clone())),
                        )
                    }
                    decl::Decl::Local(index) => {
                        Ok(Expr::new(ast.location, ExprKind::LocalRef(*index))
                            .with_type(locals[*index].typ.clone()))
                    }
                    decl::Decl::Global(index) => {
                        Ok(Expr::new(ast.location, ExprKind::GlobalRef(*index))
                            .with_type(locals[*index].typ.clone()))
                    }
                    decl::Decl::Param(typ, index) => {
                        Ok(Expr::new(ast.location, ExprKind::ParamRef(*index))
                            .with_type(typ.clone()))
                    }
                    // Struct (constructor)
                    // Enum (constructor)
                    _ => todo!("Ident: {:?}", decl),
                }
            }
            None => {
                let name = decls.symbols.resolve(*symbol);
                Err(CompilationError::UnknownSymbol(ast.location, name))
            }
        },

        NodeKind::QName(_name) => panic!("Should already be resolved"),

        NodeKind::BinaryExpr { op, lhs, rhs } => {
            let lhs_expr = build_exprs(lhs, scope, decls, locals, inference)?;
            let rhs_expr = build_exprs(rhs, scope, decls, locals, inference)?;
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

        NodeKind::Assign { lhs, rhs } => {
            let lhs_expr = build_exprs(lhs, scope, decls, locals, inference)?;
            let rhs_expr = build_exprs(rhs, scope, decls, locals, inference)?;

            inference.add_constraint(
                lhs_expr.typ.clone(),
                rhs_expr.typ.clone(),
                lhs_expr.location,
            );
            Ok(Expr::new(
                ast.location,
                ExprKind::Assign {
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(rhs_expr),
                },
            )
            .with_type(Type::Void)) // Don't support chained assignments.
        }

        NodeKind::AssignOp { op, lhs, rhs } => {
            let lhs_expr = build_exprs(lhs, scope, decls, locals, inference)?;
            let rhs_expr = build_exprs(rhs, scope, decls, locals, inference)?;
            match op {
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
                }

                // crate::oper::BinaryOp::LogAnd | crate::oper::BinaryOp::LogOr => {
                //     // Both sides must be boolean.
                //     inference.add_constraint(
                //         Type::Boolean,
                //         lhs_expr.typ.clone(),
                //         lhs_expr.location,
                //     );
                //     inference.add_constraint(
                //         Type::Boolean,
                //         rhs_expr.typ.clone(),
                //         rhs_expr.location,
                //     );
                // }
                crate::oper::BinaryOp::Shl | crate::oper::BinaryOp::Shr => todo!(),

                _ => panic!("Invalid augmented assignment operator: {:?}", op),
            };

            Ok(Expr::new(
                ast.location,
                ExprKind::AssignOp {
                    op: *op,
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(rhs_expr),
                },
            )
            .with_type(Type::Void)) // Don't support chained assignments.
        }

        NodeKind::UnaryExpr { op, arg } => {
            let arg_expr = build_exprs(arg, scope, decls, locals, inference)?;
            let ty = match op {
                crate::oper::UnaryOp::Not => {
                    inference.add_constraint(
                        Type::Boolean,
                        arg_expr.typ.clone(),
                        arg_expr.location,
                    );
                    Type::Boolean
                }
                crate::oper::UnaryOp::Neg => arg_expr.typ.clone(),
                crate::oper::UnaryOp::BitNot => arg_expr.typ.clone(),
            };

            Ok(Expr::new(
                ast.location,
                ExprKind::UnaryExpr {
                    op: *op,
                    arg: Box::new(arg_expr),
                },
            )
            .with_type(ty))
        }

        NodeKind::FieldName(base, fname) => {
            let base_expr = build_exprs(base, scope, decls, locals, inference)?;
            match base_expr.typ.clone() {
                Type::Struct(stype) => {
                    let field = stype.fields.iter().find(|field| field.name == *fname);
                    if let Some(field) = field {
                        Ok(Expr::new(
                            ast.location,
                            ExprKind::Field(Box::new(base_expr), field.index),
                        )
                        .with_type(field.typ.clone()))
                    } else {
                        Err(CompilationError::UnknownField(
                            ast.location,
                            decls.symbols.resolve(stype.name),
                            decls.symbols.resolve(*fname),
                        ))
                    }
                }
                _ => Err(CompilationError::NoFields(
                    ast.location,
                    base_expr.typ.clone(),
                )),
            }
        }

        NodeKind::FieldIndex(base, index) => {
            let base_expr = build_exprs(base, scope, decls, locals, inference)?;
            match base_expr.typ.clone() {
                Type::TupleStruct(tstype) => {
                    if *index >= tstype.fields.len() {
                        return Err(CompilationError::InvalidIndex(
                            ast.location,
                            decls.symbols.resolve(tstype.name),
                            *index,
                        ));
                    }
                    let field = tstype.fields[*index].clone();
                    Ok(
                        Expr::new(ast.location, ExprKind::Index(Box::new(base_expr), *index))
                            .with_type(field),
                    )
                }
                _ => Err(CompilationError::NoFields(
                    ast.location,
                    base_expr.typ.clone(),
                )),
            }
        }

        NodeKind::Empty => Ok(Expr::new(ast.location, ExprKind::Empty)),
        NodeKind::Decl(decl) => match decl {
            crate::ast::ASTDecl::Let {
                name,
                typ,
                value,
                is_const,
                visibility,
                ..
            } => {
                let name_str = decls.symbols.resolve(*name);
                let typ = match typ {
                    Some(typ) => Some(resolve_types(decls, scope, typ)?),
                    None => None,
                };
                let value_expr = match value {
                    Some(value) => Some(Box::new(build_exprs(
                        value, scope, decls, locals, inference,
                    )?)),
                    None => None,
                };
                let ty = match (typ, &value_expr) {
                    (Some(typ), Some(value)) => {
                        inference.add_constraint(typ.clone(), value.typ.clone(), value.location);
                        typ
                    }
                    (Some(typ), None) => typ,
                    (None, Some(value)) => value.typ.clone(),
                    (None, None) => {
                        return Err(CompilationError::MissingType(ast.location, name_str))
                    }
                };

                let index = locals.len();
                let local = decl::LocalDecl {
                    location: ast.location,
                    visibility: *visibility,
                    name: *name,
                    typ: ty.clone(),
                    is_const: *is_const,
                    index,
                    local_index: 0,
                };
                locals.push(local);
                scope.insert(*name, decl::Decl::Local(index));
                Ok(
                    Expr::new(ast.location, ExprKind::LocalDecl(index, value_expr))
                        .with_type(Type::Void),
                )
            }
            _ => todo!("Decl: {:?}", decl),
        },
        NodeKind::Block(stmts, result) => {
            let mut stmt_exprs = Vec::new();
            for stmt in *stmts {
                let stmt_expr = build_exprs(stmt, scope, decls, locals, inference)?;
                stmt_exprs.push(stmt_expr);
            }

            let result_expr = match result {
                Some(result) => Some(Box::new(build_exprs(
                    result, scope, decls, locals, inference,
                )?)),
                None => None,
            };
            let result_type = result_expr
                .as_ref()
                .map(|expr| expr.typ.clone())
                .unwrap_or(Type::Void);
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
            let to_typ = resolve_types(decls, scope, typ)?;
            let mut arg_expr = build_exprs(arg, scope, decls, locals, &mut infer)?;
            infer.solve_constraints()?;

            if to_typ == arg_expr.typ {
                Ok(arg_expr)
            } else if to_typ.is_number() && arg_expr.typ.is_number() {
                assign_types(&mut arg_expr, &infer)?;
                Ok(Expr::new(ast.location, ExprKind::Cast(Box::new(arg_expr))).with_type(to_typ))
            } else {
                Err(CompilationError::InvalidCast(
                    ast.location,
                    to_typ.clone(),
                    arg_expr.typ.clone(),
                ))
            }
        }

        NodeKind::Call(func, args) => {
            let func_expr = build_exprs(func, scope, decls, locals, inference)?;
            let mut arg_exprs = Vec::new();
            for arg in args.iter() {
                let arg_expr = build_exprs(arg, scope, decls, locals, inference)?;
                arg_exprs.push(arg_expr);
            }

            // let ret_type = inference.fresh_typevar();
            let fty = match &func_expr.typ {
                Type::Function(fty) => fty.clone(),
                _ => {
                    return Err(CompilationError::NotCallable(ast.location));
                }
            };

            if fty.params.len() != arg_exprs.len() {
                return Err(CompilationError::IncorrectNumberOfArguments(
                    ast.location,
                    fty.params.len(),
                    arg_exprs.len(),
                ));
            }

            for (param, arg) in fty.params.iter().zip(arg_exprs.iter()) {
                inference.add_constraint(param.typ.clone(), arg.typ.clone(), arg.location);
            }

            // inference.add_constraint(ret_type.clone(), fty.ret.clone(), ast.location);
            // inference.solve_constraints()?;
            for arg in arg_exprs.iter_mut() {
                assign_types(arg, inference)?;
            }

            let call = ExprKind::Call(Box::new(func_expr), arg_exprs);
            Ok(Expr::new(ast.location, call).with_type(fty.ret.clone()))
        }

        _ => {
            panic!("Invalid AST node for expression: {:?}", ast.kind);
        }
    }
}
