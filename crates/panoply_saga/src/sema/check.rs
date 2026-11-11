//! Checks a solved typed tree: every type known, and each operator given types it accepts.

use super::{
    expr::{Expr, ExprKind},
    types::{ExprType, TypeTable},
};
use crate::{
    compiler::CompilationError,
    syntax::oper::{BinaryOp, UnaryOp},
};

/// Whether `op` accepts operands of type `typ`.
pub(crate) fn binary_op_accepts(op: BinaryOp, typ: &ExprType) -> bool {
    match op {
        BinaryOp::Add
        | BinaryOp::Sub
        | BinaryOp::Mul
        | BinaryOp::Div
        | BinaryOp::Mod
        | BinaryOp::Lt
        | BinaryOp::Le
        | BinaryOp::Gt
        | BinaryOp::Ge => typ.is_number(),
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            typ.is_integer()
        }
        BinaryOp::Eq | BinaryOp::Ne => typ.is_number() || *typ == ExprType::Boolean,
        BinaryOp::LogAnd | BinaryOp::LogOr => *typ == ExprType::Boolean,
    }
}

/// Whether `as` converts `from` to `to`: numbers to numbers, and booleans to integers.
pub(crate) fn cast_allowed(from: &ExprType, to: &ExprType) -> bool {
    from == to
        || (from.is_number() && to.is_number())
        || (*from == ExprType::Boolean && to.is_integer())
}

pub(crate) fn check_types(expr: &Expr, types: &TypeTable) -> Result<(), CompilationError> {
    let typ = types.get(expr.typ);
    if matches!(typ, ExprType::Infer(_)) {
        return Err(CompilationError::UnresolvedType(expr.location));
    }
    match &expr.kind {
        ExprKind::Empty
        | ExprKind::ConstInteger(_)
        | ExprKind::ConstFloat(_)
        | ExprKind::ConstBool(_)
        | ExprKind::ConstString(_)
        | ExprKind::HostFunctionRef(_)
        | ExprKind::ScriptFunctionRef(_)
        | ExprKind::LocalRef(_)
        | ExprKind::GlobalRef(_)
        | ExprKind::ParamRef(_) => {}

        ExprKind::HostMethodRef(base, _)
        | ExprKind::Field(base, _)
        | ExprKind::NativeProp(base, _) => {
            check_types(base, types)?;
        }

        ExprKind::BinaryExpr { op, lhs, rhs } => {
            check_types(lhs, types)?;
            check_types(rhs, types)?;
            let operand = types.get(lhs.typ);
            if !binary_op_accepts(*op, operand) {
                return Err(CompilationError::InvalidBinaryOpType(
                    expr.location,
                    *op,
                    operand.clone(),
                    types.get(rhs.typ).clone(),
                ));
            }
        }

        ExprKind::UnaryExpr { op, arg } => {
            check_types(arg, types)?;
            let operand = types.get(arg.typ);
            let accepted = match op {
                UnaryOp::Not => *operand == ExprType::Boolean,
                UnaryOp::Neg => operand.is_number(),
                UnaryOp::BitNot => operand.is_integer(),
            };
            if !accepted {
                return Err(CompilationError::InvalidUnaryOpType(
                    expr.location,
                    *op,
                    operand.clone(),
                ));
            }
        }

        ExprKind::Assign { lhs, rhs } => {
            check_types(lhs, types)?;
            check_types(rhs, types)?;
        }

        ExprKind::AssignOp { op, lhs, rhs } => {
            check_types(lhs, types)?;
            check_types(rhs, types)?;
            let operand = types.get(lhs.typ);
            let arithmetic = !matches!(
                op,
                BinaryOp::LogAnd
                    | BinaryOp::LogOr
                    | BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
            );
            if !arithmetic {
                return Err(CompilationError::Internal(
                    expr.location,
                    "assignment operator",
                ));
            }
            if !binary_op_accepts(*op, operand) {
                return Err(CompilationError::InvalidBinaryOpType(
                    expr.location,
                    *op,
                    operand.clone(),
                    types.get(rhs.typ).clone(),
                ));
            }
        }

        ExprKind::Cast(arg) => {
            check_types(arg, types)?;
            let from = types.get(arg.typ);
            if !cast_allowed(from, typ) {
                return Err(CompilationError::InvalidCast(
                    expr.location,
                    typ.clone(),
                    from.clone(),
                ));
            }
        }

        ExprKind::Call(func, args) => {
            check_types(func, types)?;
            for arg in args.iter() {
                check_types(arg, types)?;
            }
        }

        ExprKind::Block(stmts, result) => {
            for stmt in stmts.iter() {
                check_types(stmt, types)?;
            }
            if let Some(result) = result {
                check_types(result, types)?;
            }
        }

        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            check_types(condition, types)?;
            check_types(then_branch, types)?;
            if let Some(e) = else_branch {
                check_types(e, types)?;
            }
        }
    }
    Ok(())
}
