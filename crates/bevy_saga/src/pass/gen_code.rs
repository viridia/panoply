use std::result;

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

pub(crate) fn gen_module<'ast>(unit: &mut CompilationUnit) -> Result<(), CompilationError> {
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
                let mut func = walrus::FunctionBuilder::new(
                    &mut unit.module.types,
                    &[ret_type],
                    param_types.as_slice(),
                );

                func.func_body()
                    .i32_const(1)
                    .i32_const(2)
                    .binop(walrus::ir::BinaryOp::I32Add);
                let test_fn = func.finish(Vec::new(), &mut unit.module.funcs);

                // Export the function.
                unit.module.exports.add(name_str.as_str(), test_fn);
            }
            decl::DeclKind::Struct(_) => todo!(),
            decl::DeclKind::Enum(_) => todo!(),
        }
    }

    Ok(())
}

fn gen_exprs<'a>(unit: &'a mut CompilationUnit, expr: &'a Expr) -> Result<(), CompilationError> {
    match &expr.kind {
        ExprKind::Empty => todo!(),
        ExprKind::ConstInteger(_) => todo!(),
        ExprKind::ConstFloat(_) => todo!(),
        ExprKind::String(symbol) => todo!(),
        ExprKind::Ident(symbol) => todo!(),
        ExprKind::BinaryExpr { op, lhs, rhs } => todo!(),
        ExprKind::Block(vec, expr) => todo!(),
    }
}

fn gen_type<'a>(typ: &'a Type) -> walrus::ValType {
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
