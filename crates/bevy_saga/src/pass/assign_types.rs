use core::result;

use crate::{
    compiler::CompilationError,
    decl,
    expr::{Expr, ExprKind},
    types::Type,
};

use super::type_inference::TypeInference;

pub(crate) fn assign_types(
    expr: &mut Expr,
    inference: &TypeInference,
) -> Result<(), CompilationError> {
    match &mut expr.kind {
        ExprKind::Empty => {}
        ExprKind::ConstInteger(_) => {
            inference.replace_type_vars(&mut expr.typ);
        }
        ExprKind::ConstFloat(_) => {
            inference.replace_type_vars(&mut expr.typ);
        }
        ExprKind::ConstBool(_) => {}
        ExprKind::ConstString(_symbol) => todo!(),
        ExprKind::FunctionRef(_) => {}
        ExprKind::LocalRef(_) | ExprKind::ParamRef(_) => {}
        ExprKind::LocalDecl(_, ref mut init) => {
            if let Some(init) = init {
                assign_types(init, inference)?;
            }
        }
        ExprKind::BinaryExpr { op, lhs, rhs } => {
            assign_types(lhs, inference)?;
            assign_types(rhs, inference)?;
            match op {
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

                        _ => {
                            return Err(CompilationError::InvalidBinaryOpType(
                                expr.location,
                                *op,
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

                        _ => {
                            return Err(CompilationError::InvalidBinaryOpType(
                                expr.location,
                                *op,
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

        // No need to traverse here, this has already been done.
        ExprKind::Cast(_arg) => {}
        ExprKind::Call(func, args) => {
            assign_types(func, inference)?;
            for arg in args.iter_mut() {
                assign_types(arg, inference)?;
            }
        }

        ExprKind::Block(ref mut stmts, ref mut result) => {
            for stmt in stmts.iter_mut() {
                assign_types(stmt, inference)?;
            }
            if let Some(ref mut result) = result {
                assign_types(result.as_mut(), inference)?;
            }
        }
    }
    Ok(())
}
