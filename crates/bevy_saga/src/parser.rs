use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "saga.pest"]
pub struct SagaParser;

#[cfg(test)]
mod tests {
    use crate::{
        ast::{self, FloatSuffix, IntegerSuffix},
        compiler::CompilationUnit,
        oper,
        pass::{self, assign_types},
        Type,
    };
    use pest::Parser;

    use super::*;

    #[test]
    fn parse_integer() {
        let pairs = SagaParser::parse(Rule::integer, "20").unwrap_or_else(|e| panic!("{}", e));
        let arena = bumpalo::Bump::new();
        let node = pass::build_ast(&arena, pairs);
        assert!(matches!(
            node.kind,
            ast::NodeKind::ConstInteger("20", IntegerSuffix::Unsized)
        ));
        let mut unit = CompilationUnit::new();
        let mut inference: pass::TypeInference = Default::default();
        let expr = pass::build_exprs(&mut unit, node, &mut inference);
        assert_eq!(expr.to_string(), "20");
        inference.solve_constraints().unwrap();
        let span = expr.location.as_span("20").unwrap();
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 2);
        assert_eq!(span.lines().next(), Some("20"));
    }

    #[test]
    fn parse_float() {
        let pairs = SagaParser::parse(Rule::float, "20.0").unwrap_or_else(|e| panic!("{}", e));
        let arena = bumpalo::Bump::new();
        let node = pass::build_ast(&arena, pairs);
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
        let pairs =
            SagaParser::parse(Rule::binary, "20.0 + 10.0").unwrap_or_else(|e| panic!("{}", e));
        let arena = bumpalo::Bump::new();
        let node = pass::build_ast(&arena, pairs);
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
        let pairs =
            SagaParser::parse(Rule::binary, "20.0 + 10.0 * 0").unwrap_or_else(|e| panic!("{}", e));
        let arena = bumpalo::Bump::new();
        let node = pass::build_ast(&arena, pairs);
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
}
