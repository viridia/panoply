use std::sync::Arc;

use bevy::utils::HashMap;

use crate::{
    ast::{ASTNode, NodeKind},
    compiler::{CompilationError, CompilationUnit},
    expr::{Expr, ExprKind},
    types::{Type, TypeVarId},
    TypeError,
};

#[derive(Default)]
pub(crate) struct TypeInference {
    /// Equivalence constraints.
    constraints: Vec<(Type, Type)>,

    /// Type variable substitutions.
    substitutions: HashMap<TypeVarId, Type>,
}

impl TypeInference {
    fn replace_type_vars(&self, ty: &mut Type) {
        *ty = self.substitute(ty);
    }

    fn substitute(&self, ty: &Type) -> Type {
        match ty {
            Type::Infer(id) => {
                if let Some(substituted_type) = self.substitutions.get(id) {
                    self.substitute(substituted_type)
                } else {
                    ty.clone()
                }
            }

            Type::Error(_) => {
                // Do nothing
                ty.clone()
            }

            Type::Tuple(types) => {
                // TODO: Return the original type if no substitutions are made.
                let mut new_types = Vec::with_capacity(types.len());
                for t in types.iter() {
                    new_types.push(self.substitute(t));
                }
                Type::Tuple(Arc::from(new_types))
            }

            // TODO: Return the original type if no substitutions are made.
            Type::Array(ty) => Type::Array(Arc::new(self.substitute(ty))),

            _ => ty.clone(),
        }
    }

    fn occurs_check(&self, var_id: TypeVarId, ty: &Type) -> bool {
        match ty {
            Type::Infer(id) => {
                if *id == var_id {
                    return true;
                }
                // Check if this type variable has a substitution
                if let Some(substituted_type) = self.substitutions.get(id) {
                    return self.occurs_check(var_id, substituted_type);
                }
                false
            }
            _ => false,
        }
    }

    fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), TypeError> {
        let t1 = self.substitute(t1);
        let t2 = self.substitute(t2);

        if t1 == t2 {
            return Ok(());
        }

        match (&t1, &t2) {
            (Type::Error(err), _) | (_, Type::Error(err)) => Err(err.clone()),

            (Type::Infer(id), ty) | (ty, Type::Infer(id)) => {
                if self.occurs_check(*id, ty) {
                    return Err(TypeError::RecursiveType(Arc::new(ty.clone())));
                } else {
                    self.substitutions.insert(*id, ty.clone());
                }
                Ok(())
            }

            (Type::I32, Type::IUnsized) | (Type::IUnsized, Type::I32) => Ok(()),
            (Type::I64, Type::IUnsized) | (Type::IUnsized, Type::I64) => Ok(()),

            (Type::Tuple(types1), Type::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return Err(TypeError::MismatchedTypes(
                        Arc::new(t1.clone()),
                        Arc::new(t2.clone()),
                    ));
                }
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }

            (Type::Array(t1), Type::Array(t2)) => self.unify(t1, t2),

            // (Type::Int, Type::Float) | (Type::Float, Type::Int) => {
            //     Ok(()) // Allow implicit conversion between int and float
            // }
            _ => Err(TypeError::MismatchedTypes(
                Arc::new(t1.clone()),
                Arc::new(t2.clone()),
            )),
        }
    }

    pub(crate) fn solve_constraints(&mut self) -> Result<(), TypeError> {
        while let Some((t1, t2)) = self.constraints.pop() {
            self.unify(&t1, &t2)?;
        }
        Ok(())
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

                crate::oper::BinaryOp::Shl => todo!(),
                crate::oper::BinaryOp::Shr => todo!(),

                crate::oper::BinaryOp::Eq
                | crate::oper::BinaryOp::Ne
                | crate::oper::BinaryOp::Lt
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
                crate::oper::BinaryOp::LogAnd => todo!(),
                crate::oper::BinaryOp::LogOr => todo!(),
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
                crate::oper::BinaryOp::Shl => todo!(),
                crate::oper::BinaryOp::Shr => todo!(),
                crate::oper::BinaryOp::Eq => todo!(),
                crate::oper::BinaryOp::Ne => todo!(),
                crate::oper::BinaryOp::Lt => todo!(),
                crate::oper::BinaryOp::Le => todo!(),
                crate::oper::BinaryOp::Gt => todo!(),
                crate::oper::BinaryOp::Ge => todo!(),
            }
        }
    }
    Ok(())
}
