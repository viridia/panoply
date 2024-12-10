use crate::{
    compiler::CompilationError,
    expr::{Expr, ExprKind},
    types::Type,
    TypeError,
};

use super::type_inference::TypeInference;

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
