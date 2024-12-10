use crate::{
    ast::{ASTNode, NodeKind},
    compiler::{CompilationError, CompilationUnit},
    expr::{Expr, ExprKind},
    types::Type,
    TypeError,
};

use super::type_inference::TypeInference;

pub(crate) fn build_unit<'a>(
    unit: &'a mut CompilationUnit,
    ast: &'a ASTNode<'a>,
    inference: &mut TypeInference,
) {
    if let NodeKind::Unit(decls) = &ast.kind {
        for decl in *decls {
            match &decl.kind {
                NodeKind::Decl(d) => todo!(),
                _ => panic!("Invalid AST node for declaration: {:?}", decl.kind),
            }
        }
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
            let id = unit.next_expr_id();
            let value = value.parse::<i64>().unwrap();
            let typ = match suffix {
                &crate::ast::IntegerSuffix::Unsized => {
                    let ty = Type::Infer(unit.next_typevar_id());
                    if value > i32::MAX as i64 || value < i32::MIN as i64 {
                        inference.constraints.push((ty.clone(), Type::I64));
                    } else {
                        inference.constraints.push((ty.clone(), Type::I32));
                    }
                    ty
                }
                crate::ast::IntegerSuffix::I32 => Type::I32,
                crate::ast::IntegerSuffix::I64 => Type::I64,
            };
            Expr::new(id, ast.location, ExprKind::ConstInteger(value)).with_type(typ)
        }

        NodeKind::ConstFloat(value, suffix) => {
            let id = unit.next_expr_id();
            let value = value.parse::<f64>().unwrap();
            let typ = match suffix {
                crate::ast::FloatSuffix::F32 => Type::F32,
                crate::ast::FloatSuffix::F64 => Type::F64,
                _ => unreachable!(),
            };
            Expr::new(id, ast.location, ExprKind::ConstFloat(value)).with_type(typ)
        }

        NodeKind::String(_) => todo!(),
        NodeKind::Ident(_) => todo!(),

        NodeKind::BinaryExpr { op, lhs, rhs } => {
            let id = unit.next_expr_id();
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
                    let ty = Type::Infer(unit.next_typevar_id());
                    // Both sides must be the same, which is also the result type.
                    inference
                        .constraints
                        .push((ty.clone(), lhs_expr.typ.clone()));
                    inference
                        .constraints
                        .push((ty.clone(), rhs_expr.typ.clone()));
                    ty
                }

                crate::oper::BinaryOp::LogAnd | crate::oper::BinaryOp::LogOr => {
                    // Both sides must be boolean.
                    inference
                        .constraints
                        .push((lhs_expr.typ.clone(), Type::Boolean));
                    inference
                        .constraints
                        .push((rhs_expr.typ.clone(), Type::Boolean));
                    Type::Boolean
                }

                crate::oper::BinaryOp::Shl | crate::oper::BinaryOp::Shr => todo!(),

                crate::oper::BinaryOp::Eq | crate::oper::BinaryOp::Ne => todo!(),

                crate::oper::BinaryOp::Lt
                | crate::oper::BinaryOp::Le
                | crate::oper::BinaryOp::Gt
                | crate::oper::BinaryOp::Ge => {
                    // Both sides must be the same.
                    inference
                        .constraints
                        .push((lhs_expr.typ.clone(), rhs_expr.typ.clone()));
                    Type::Boolean
                }
            };

            Expr::new(
                id,
                ast.location,
                ExprKind::BinaryExpr {
                    op: *op,
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(rhs_expr),
                },
            )
            .with_type(ty)
        }

        _ => {
            panic!("Invalid AST node for expression: {:?}", ast.kind);
        }
    }
}

pub(crate) fn assign_types(
    expr: &mut Expr,
    inference: &TypeInference,
) -> Result<(), CompilationError> {
    match expr.kind {
        ExprKind::ConstInteger(_) => {
            inference.replace_type_vars(&mut expr.typ);
        }
        ExprKind::ConstFloat(_) => {
            inference.replace_type_vars(&mut expr.typ);
        }
        ExprKind::String(symbol) => todo!(),
        ExprKind::Ident(symbol) => todo!(),
        ExprKind::BinaryExpr {
            op,
            ref mut lhs,
            ref mut rhs,
        } => {
            assign_types(lhs, inference)?;
            assign_types(rhs, inference)?;
            match op {
                crate::oper::BinaryOp::MemberAccess => todo!(),

                crate::oper::BinaryOp::Add
                | crate::oper::BinaryOp::Sub
                | crate::oper::BinaryOp::Mul
                | crate::oper::BinaryOp::Div
                | crate::oper::BinaryOp::Mod => {
                    let ty = inference.substitute(&expr.typ);
                    match ty {
                        Type::I32 | Type::I64 | Type::F32 | Type::F64 => {
                            expr.typ = ty;
                        }

                        Type::Error(TypeError::RecursiveType(_)) => {
                            return Err(CompilationError::RecursiveType(expr.location, ty.clone()))
                        }

                        _ => {
                            return Err(CompilationError::InvalidBinaryOpType(
                                expr.location,
                                op,
                                lhs.typ.clone(),
                                rhs.typ.clone(),
                            ))
                        }
                    }
                }

                crate::oper::BinaryOp::LogAnd | crate::oper::BinaryOp::LogOr => todo!(),

                crate::oper::BinaryOp::BitAnd
                | crate::oper::BinaryOp::BitOr
                | crate::oper::BinaryOp::BitXor => {
                    let ty = inference.substitute(&expr.typ);
                    match ty {
                        Type::I32 | Type::I64 => {
                            expr.typ = ty;
                        }

                        Type::Error(TypeError::RecursiveType(_)) => {
                            return Err(CompilationError::RecursiveType(expr.location, ty.clone()))
                        }

                        _ => {
                            return Err(CompilationError::InvalidBinaryOpType(
                                expr.location,
                                op,
                                lhs.typ.clone(),
                                rhs.typ.clone(),
                            ))
                        }
                    }
                }
                crate::oper::BinaryOp::Shl | crate::oper::BinaryOp::Shr => todo!(),

                crate::oper::BinaryOp::Eq | crate::oper::BinaryOp::Ne => todo!(),

                crate::oper::BinaryOp::Lt
                | crate::oper::BinaryOp::Le
                | crate::oper::BinaryOp::Gt
                | crate::oper::BinaryOp::Ge => todo!(),
            }
        }
    }
    Ok(())
}
