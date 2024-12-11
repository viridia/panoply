use thiserror::Error;

use std::cell::RefCell;

use bevy::utils::HashMap;

use crate::{
    expr::{Expr, Symbol},
    location::TokenLocation,
    oper::BinaryOp,
    types::TypeVarId,
    Type,
};

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
}

/// Contains all of the information needed to compile a single script file.
pub struct CompilationUnit {
    pub(crate) next_expr_id: usize,
    pub(crate) next_typevar_id: usize,
    pub(crate) symbols: RefCell<Vec<String>>,
    pub(crate) decls: HashMap<Symbol, Decl>,
}

impl Default for CompilationUnit {
    fn default() -> Self {
        Self {
            next_expr_id: 0,
            next_typevar_id: 0,
            symbols: RefCell::new(Vec::new()),
            decls: HashMap::default(),
        }
    }
}

impl CompilationUnit {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn alloc_symbol(&self, value: &str) -> Symbol {
        let mut symbols = self.symbols.borrow_mut();
        let id = Symbol(symbols.len());
        symbols.push(value.to_string());
        id
    }

    pub(crate) fn next_expr_id(&mut self) -> usize {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        id
    }

    pub(crate) fn next_typevar_id(&mut self) -> TypeVarId {
        let id = self.next_typevar_id;
        self.next_typevar_id += 1;
        TypeVarId(id)
    }
}

pub(crate) struct Decl {
    // pub(crate) id: NodeId,
    pub(crate) name: Symbol,
    pub(crate) loc: TokenLocation,
    pub(crate) typ: TypeVarId,
}

pub enum DeclKind {
    Const(Type, Expr),
    Function(Type),
    Struct(Type),
    Enum(Type),
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{self, FloatSuffix, IntegerSuffix},
        compiler::CompilationUnit,
        expr::SymbolTable,
        oper,
        parser::saga_parser,
        pass::{self, assign_types},
        Type,
    };

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
        assert_eq!(err.to_string(), "Mismatched types");
    }

    #[test]
    fn parse_module() {
        let arena = bumpalo::Bump::new();
        let symbols = SymbolTable::new();
        let node = saga_parser::unit(
            r#"
            fn test() {
                1 + 2
            }"#,
            &arena,
            &symbols,
        )
        .unwrap();
        assert!(matches!(node.kind, ast::NodeKind::Unit(_)));
        match node.kind {
            ast::NodeKind::Unit(decls) => {
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
    }
}
