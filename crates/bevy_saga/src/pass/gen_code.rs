use core::result;

use crate::{
    ast::{ASTNode, DeclKind, NodeKind},
    compiler::{CompilationError, CompilationUnit},
    decl,
    expr::{Expr, ExprKind},
    types::Type,
};

use super::{
    assign_types::assign_types, resolve_types::resolve_types, type_inference::TypeInference,
};

pub(crate) fn gen_module(unit: &mut CompilationUnit) -> Result<(), CompilationError> {
    for (_, decl) in unit.root_scope.decls.iter() {
        match &decl.kind {
            decl::DeclKind::Const(_, expr) => todo!(),
            decl::DeclKind::Let(_, expr) => todo!(),
            decl::DeclKind::Param(_) => todo!(),
            decl::DeclKind::Function {
                name,
                params,
                ret,
                body,
            } => {
                let name_str = unit.symbols.resolve(*name);
                let ret_type = gen_type(ret);
                let param_types = params
                    .iter()
                    .map(|p| {
                        if let decl::DeclKind::Param(param_type) = &p.kind {
                            gen_type(param_type)
                        } else {
                            panic!("Invalid parameter declaration for function: {:?}", p);
                        }
                    })
                    .collect::<Vec<_>>();

                let mut fn_builder = walrus::FunctionBuilder::new(
                    &mut unit.module.types,
                    param_types.as_slice(),
                    &[ret_type],
                );

                let mut instr_builder = fn_builder.func_body();
                gen_expr(unit, body, &mut instr_builder)?;
                if !body.typ.is_void() {
                    instr_builder.return_();
                }
                let func = fn_builder.finish(Vec::new(), &mut unit.module.funcs);

                // Export the function.
                unit.module.exports.add(name_str.as_str(), func);
            }
            decl::DeclKind::Struct(_) => todo!(),
            decl::DeclKind::Enum(_) => todo!(),
        }
    }

    Ok(())
}

fn gen_expr<'a>(
    unit: &'a CompilationUnit,
    expr: &'a Expr,
    out: &mut walrus::InstrSeqBuilder,
) -> Result<(), CompilationError> {
    match &expr.kind {
        ExprKind::Empty => todo!(),
        ExprKind::ConstInteger(value) => {
            match expr.typ {
                Type::I32 => out.i32_const(*value as i32),
                Type::I64 => out.i64_const(*value),
                _ => panic!("Invalid integer type: {:?}", expr.typ),
            };
        }
        ExprKind::ConstFloat(value) => {
            match expr.typ {
                Type::F32 => out.f32_const(*value as f32),
                Type::F64 => out.f64_const(*value),
                _ => panic!("Invalid float type: {:?}", expr.typ),
            };
        }
        ExprKind::String(symbol) => todo!(),
        ExprKind::Ident(symbol) => todo!(),
        ExprKind::BinaryExpr { op, lhs, rhs } => {
            gen_expr(unit, lhs, out)?;
            gen_expr(unit, rhs, out)?;
            match op {
                crate::oper::BinaryOp::Add => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Add),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Add),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Add),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Add),
                        _ => panic!("Invalid type for binary addition: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Sub => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Sub),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Sub),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Sub),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Sub),
                        _ => panic!("Invalid type for binary subtraction: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Mul => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Mul),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Mul),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Mul),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Mul),
                        _ => panic!("Invalid type for binary multiplication: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Div => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32DivS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64DivS),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Div),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Div),
                        _ => panic!("Invalid type for binary division: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Mod => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32RemS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64RemS),
                        _ => panic!("Invalid type for binary modulo: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::LogAnd => {
                    // TODO: Short-circuiting.
                    match expr.typ {
                        Type::Boolean => out.binop(walrus::ir::BinaryOp::I32And),
                        _ => panic!("Invalid type for logical AND: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::LogOr => {
                    // TODO: Short-circuiting.
                    match expr.typ {
                        Type::Boolean => out.binop(walrus::ir::BinaryOp::I32Or),
                        _ => panic!("Invalid type for logical OR: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::BitAnd => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32And),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64And),
                        _ => panic!("Invalid type for bitwise AND: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::BitOr => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Or),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Or),
                        _ => panic!("Invalid type for bitwise OR: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::BitXor => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Xor),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Xor),
                        _ => panic!("Invalid type for bitwise XOR: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Shl => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Shl),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Shl),
                        _ => panic!("Invalid type for bitwise shift left: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Shr => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32ShrS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64ShrS),
                        _ => panic!("Invalid type for bitwise shift right: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Eq => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Eq),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Eq),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Eq),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Eq),
                        _ => panic!("Invalid type for equality comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Ne => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32Ne),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64Ne),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Ne),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Ne),
                        _ => panic!("Invalid type for inequality comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Lt => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32LtS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64LtS),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Lt),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Lt),
                        _ => panic!("Invalid type for less-than comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Le => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32LeS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64LeS),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Le),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Le),
                        _ => panic!(
                            "Invalid type for less-than-or-equal comparison: {:?}",
                            expr.typ
                        ),
                    };
                }
                crate::oper::BinaryOp::Gt => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32GtS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64GtS),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Gt),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Gt),
                        _ => panic!("Invalid type for greater-than comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Ge => {
                    match expr.typ {
                        Type::I32 => out.binop(walrus::ir::BinaryOp::I32GeS),
                        Type::I64 => out.binop(walrus::ir::BinaryOp::I64GeS),
                        Type::F32 => out.binop(walrus::ir::BinaryOp::F32Ge),
                        Type::F64 => out.binop(walrus::ir::BinaryOp::F64Ge),
                        _ => panic!(
                            "Invalid type for greater-than-or-equal comparison: {:?}",
                            expr.typ
                        ),
                    };
                }
            }
        }

        ExprKind::Block(vec, expr) => {
            for stmt in vec {
                gen_expr(unit, stmt, out)?;
                if !stmt.typ.is_void() {
                    out.drop();
                }
            }

            if let Some(expr) = expr {
                gen_expr(unit, expr, out)?;
            }
        }
    }

    Ok(())
}

fn gen_type(typ: &Type) -> walrus::ValType {
    match &typ {
        Type::Boolean => walrus::ValType::I32,
        Type::I32 => walrus::ValType::I32,
        Type::I64 => walrus::ValType::I64,
        Type::F32 => walrus::ValType::F32,
        Type::F64 => walrus::ValType::F64,
        Type::String => walrus::ValType::Ref(walrus::RefType::Externref),
        Type::Tuple(arc) => todo!(),
        Type::Array(arc) => todo!(),
        Type::Function { params, ret } => todo!(),
        _ => panic!("Invalid type for code generation: {:?}", typ),
    }
}
