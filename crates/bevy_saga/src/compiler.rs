use std::marker::PhantomData;

use crate::{
    decl::{Scope, SymbolTable},
    location::TokenLocation,
    oper::BinaryOp,
    parser::saga_parser,
    pass,
    types::TypeVarId,
    Type,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("Expression expected")]
    ExpectExpression(TokenLocation),
    #[error("Mismatched types: {1} {2}")]
    MismatchedTypes(TokenLocation, Type, Type),
    #[error("Recursive type: {1}")]
    RecursiveType(TokenLocation, Type),
    #[error("Invalid type for binary operator {2} to {3}")]
    InvalidBinaryOpType(TokenLocation, BinaryOp, Type, Type),
    #[error("Function redefinition: {1}")]
    FunctionRedefinition(TokenLocation, String),
}

/// Contains all of the information needed to compile a single script file.
pub struct CompilationUnit<'cu> {
    pub(crate) next_typevar_id: usize,
    pub(crate) symbols: SymbolTable,
    pub(crate) root_scope: Scope<'cu>,
}

impl<'cu> Default for CompilationUnit<'cu> {
    fn default() -> Self {
        Self {
            next_typevar_id: 0,
            symbols: SymbolTable::new(),
            root_scope: Scope::new(None),
        }
    }
}

impl<'cu> CompilationUnit<'cu> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile a script file.
    pub async fn compile(&mut self, src: &str) -> Result<(), CompilationError> {
        let arena = bumpalo::Bump::new();
        let ast = saga_parser::compilation_unit(src, &arena, &self.symbols).map_err(|e| {
            eprintln!("Error parsing script: {}", e);
            // CompilationError::ExpectExpression(TokenLocation::default())
            todo!();
        })?;
        pass::build_module_decls(self, ast)?;
        self.resolve_imports().await?;
        pass::build_module_exprs(self, ast)?;
        Ok(())
    }

    async fn resolve_imports(&mut self) -> Result<(), CompilationError> {
        // TODO: Implement
        Ok(())
    }

    pub(crate) fn next_typevar_id(&mut self) -> TypeVarId {
        let id = self.next_typevar_id;
        self.next_typevar_id += 1;
        TypeVarId(id)
    }

    pub(crate) fn fresh_typevar(&mut self) -> Type {
        Type::Infer(self.next_typevar_id())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{self, FloatSuffix, IntegerSuffix},
        compiler::CompilationUnit,
        decl::SymbolTable,
        oper,
        parser::saga_parser,
        pass::{self, assign_types},
        Type,
    };
    use futures_lite::future;
    use walrus::ModuleConfig;

    #[test]
    fn parse_integer() {
        let arena = bumpalo::Bump::new();
        let symbols = SymbolTable::new();
        let node = saga_parser::expr("20", &arena, &symbols).unwrap();
        assert!(matches!(
            node.kind,
            ast::NodeKind::ConstInteger("20", IntegerSuffix::Unsized)
        ));
        let mut unit = CompilationUnit::new();
        let mut inference: pass::TypeInference = Default::default();
        let expr = pass::build_exprs(&mut unit, node, &mut inference);
        assert_eq!(expr.to_string(), "20");
        inference.solve_constraints().unwrap();
        // let span = expr.location.as_span("20").unwrap();
        // assert_eq!(span.start(), 0);
        // assert_eq!(span.end(), 2);
        // assert_eq!(span.lines().next(), Some("20"));
    }

    #[test]
    fn parse_float() {
        let arena = bumpalo::Bump::new();
        let symbols = SymbolTable::new();
        let node = saga_parser::expr("20.0", &arena, &symbols).unwrap();
        assert!(matches!(
            node.kind,
            ast::NodeKind::ConstFloat("20.0", FloatSuffix::F32)
        ));
        let mut unit = CompilationUnit::new();
        let mut inference: pass::TypeInference = Default::default();
        let expr = pass::build_exprs(&mut unit, node, &mut inference);
        assert_eq!(expr.to_string(), "20.0");
        inference.solve_constraints().unwrap();
    }

    #[test]
    fn parse_binop_add() {
        let arena = bumpalo::Bump::new();
        let symbols = SymbolTable::new();
        let node = saga_parser::expr("20.0 + 10.0", &arena, &symbols).unwrap();
        match &node.kind {
            ast::NodeKind::BinaryExpr { op, lhs, rhs } => {
                assert_eq!(*op, oper::BinaryOp::Add);
                assert!(matches!(
                    lhs.kind,
                    ast::NodeKind::ConstFloat("20.0", FloatSuffix::F32)
                ));
                assert!(matches!(
                    rhs.kind,
                    ast::NodeKind::ConstFloat("10.0", FloatSuffix::F32)
                ));
            }
            _ => panic!(),
        }
        let mut unit = CompilationUnit::new();
        let mut inference: pass::TypeInference = Default::default();
        let mut expr = pass::build_exprs(&mut unit, node, &mut inference);
        assert_eq!(expr.to_string(), "20.0 + 10.0");
        inference.solve_constraints().unwrap();
        assign_types(&mut expr, &inference).unwrap();
        assert_eq!(expr.typ, Type::F32);
    }
    #[test]
    fn parse_binop_prec() {
        let arena = bumpalo::Bump::new();
        let symbols = SymbolTable::new();
        let node = saga_parser::expr("20.0 + 10.0 * 0", &arena, &symbols).unwrap();
        match &node.kind {
            ast::NodeKind::BinaryExpr { op, lhs, rhs } => {
                assert_eq!(*op, oper::BinaryOp::Add);
                assert!(matches!(
                    lhs.kind,
                    ast::NodeKind::ConstFloat("20.0", FloatSuffix::F32)
                ));
                match &rhs.kind {
                    ast::NodeKind::BinaryExpr { op, lhs, rhs } => {
                        assert_eq!(*op, oper::BinaryOp::Mul);
                        assert!(matches!(
                            lhs.kind,
                            ast::NodeKind::ConstFloat("10.0", FloatSuffix::F32)
                        ));
                        assert!(matches!(
                            rhs.kind,
                            ast::NodeKind::ConstInteger("0", IntegerSuffix::Unsized)
                        ));
                    }
                    _ => panic!(),
                }
                // assert!(matches!(rhs.value, ast::NodeValue::ConstF64(10.0)));
            }
            _ => panic!(),
        }
        let mut unit = CompilationUnit::new();
        let mut inference: pass::TypeInference = Default::default();
        let expr = pass::build_exprs(&mut unit, node, &mut inference);
        assert_eq!(expr.to_string(), "20.0 + 10.0 * 0");
        let err = inference.solve_constraints().unwrap_err();
        assert_eq!(err.to_string(), "Mismatched types: f32 i32");
    }

    #[test]
    fn parse_module() {
        let arena = bumpalo::Bump::new();
        let symbols = SymbolTable::new();
        let node = saga_parser::compilation_unit(
            r#"
            fn test() -> i32 {
                1 + 2
            }"#,
            &arena,
            &symbols,
        )
        .unwrap();
        assert!(matches!(node.kind, ast::NodeKind::Program(_)));
        match node.kind {
            ast::NodeKind::Program(decls) => {
                assert_eq!(decls.len(), 1);
                let decl = decls[0];
                assert!(matches!(decl.kind, ast::NodeKind::Decl(_)));
            }
            _ => panic!(),
        }
        // let mut unit = CompilationUnit::new();
        // let mut inference: pass::TypeInference = Default::default();
        // let expr = pass::build_exprs(&mut unit, node, &mut inference);
        // assert_eq!(expr.to_string(), "20");
        // inference.solve_constraints().unwrap();
        // let span = expr.location.as_span("20").unwrap();
        // assert_eq!(span.start(), 0);
        // assert_eq!(span.end(), 2);
        // assert_eq!(span.lines().next(), Some("20"));
        let config = ModuleConfig::default();
        let mut module = walrus::Module::with_config(config);

        let mut test_fn = walrus::FunctionBuilder::new(
            &mut module.types,
            &[walrus::ValType::I32],
            &[walrus::ValType::I32],
        );

        test_fn
            .func_body()
            .i32_const(1)
            .i32_const(2)
            .binop(walrus::ir::BinaryOp::I32Add);
        let test_fn = test_fn.finish(Vec::new(), &mut module.funcs);

        // Export the `test` function.
        module.exports.add("test", test_fn);
        let wasm = module.emit_wasm();
    }

    #[test]
    fn test_compiler() {
        let mut unit = CompilationUnit::new();
        future::block_on(unit.compile("fn test() -> i32 { 1 + 2 }")).unwrap();
    }
}
