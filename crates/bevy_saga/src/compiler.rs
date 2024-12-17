use std::path::Path;

use crate::{
    decl::{Decl, Decls, InternedSymbols, Scope},
    location::TokenLocation,
    oper::BinaryOp,
    parser::saga_parser,
    pass, Type,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("Expression expected")]
    ExpectExpression(TokenLocation),
    #[error("Declaration expected")]
    ExpectDeclaration(TokenLocation),
    #[error("Statement expected")]
    ExpectStatement(TokenLocation),
    #[error("Expected {1}")]
    Expected(TokenLocation, String),
    #[error("Cannot assign type {2} to {1}")]
    MismatchedTypes(TokenLocation, Type, Type),
    #[error("Cannot convert type from {2} to {1}")]
    InvalidCast(TokenLocation, Type, Type),
    #[error("Call expression requires function")]
    NotCallable(TokenLocation),
    #[error("Function expects {1} arguments, got {2}")]
    IncorrectNumberOfArguments(TokenLocation, usize, usize),
    #[error("Recursive type: {1}")]
    RecursiveType(TokenLocation, Type),
    #[error("Cannot infer type for '{1}'")]
    MissingType(TokenLocation, String),
    #[error("Unknown type: {1}")]
    UnknownType(TokenLocation, String),
    #[error("Can't find the name '{1}' in this scope")]
    UnknownSymbol(TokenLocation, String),
    #[error("Invalid type for binary operator {2} to {3}")]
    InvalidBinaryOpType(TokenLocation, BinaryOp, Type, Type),
    #[error("Function redefinition: {1}")]
    FunctionRedefinition(TokenLocation, String),
}

impl CompilationError {
    pub fn location(&self) -> TokenLocation {
        match self {
            CompilationError::ExpectExpression(loc)
            | CompilationError::ExpectDeclaration(loc)
            | CompilationError::ExpectStatement(loc)
            | CompilationError::Expected(loc, _)
            | CompilationError::MismatchedTypes(loc, _, _)
            | CompilationError::InvalidCast(loc, _, _)
            | CompilationError::NotCallable(loc)
            | CompilationError::IncorrectNumberOfArguments(loc, _, _)
            | CompilationError::RecursiveType(loc, _)
            | CompilationError::MissingType(loc, _)
            | CompilationError::UnknownType(loc, _)
            | CompilationError::UnknownSymbol(loc, _)
            | CompilationError::InvalidBinaryOpType(loc, _, _, _)
            | CompilationError::FunctionRedefinition(loc, _) => *loc,
        }
    }
}

/// Contains all of the information needed to compile a single script file.
pub struct CompilationUnit<'cu> {
    path: &'cu str,
    src: &'cu str,

    /// Interned symbols
    pub(crate) symbols: InternedSymbols,

    /// All declarations, both global and local.
    pub(crate) decls: Decls,
    pub(crate) module: wasm_encoder::Module,
}

impl<'cu> CompilationUnit<'cu> {
    pub fn new(path: &'cu str, src: &'cu str) -> Self {
        Self {
            path,
            src,
            symbols: InternedSymbols::new(),
            decls: Decls::new(),
            module: wasm_encoder::Module::new(),
        }
    }

    pub fn filename(&self) -> &'cu str {
        Path::new(self.path).file_name().unwrap().to_str().unwrap()
    }

    /// Compile a script file.
    pub async fn compile(&mut self) -> Result<(), CompilationError> {
        let arena = bumpalo::Bump::new();
        let ast =
            saga_parser::compilation_unit(self.src, &arena, &self.symbols).map_err(|err| {
                let location = TokenLocation::new(err.location.offset, err.location.offset + 1);
                for token in err.expected.tokens() {
                    match token {
                        "expression" => return CompilationError::ExpectExpression(location),
                        "declaration" => return CompilationError::ExpectDeclaration(location),
                        "statement" => return CompilationError::ExpectStatement(location),
                        _ => {}
                    }
                }
                let tokens = err.expected.to_string();
                CompilationError::Expected(location, tokens)
            })?;

        let mut intrinsic_scope = Scope::new(None);
        fn define_type(symbols: &InternedSymbols, scope: &mut Scope, name: &str, ty: Type) {
            let sym = symbols.intern(name);
            scope.insert(sym, Decl::Type(ty));
        }

        define_type(&self.symbols, &mut intrinsic_scope, "i32", Type::I32);
        define_type(&self.symbols, &mut intrinsic_scope, "i64", Type::I64);
        define_type(&self.symbols, &mut intrinsic_scope, "f32", Type::F32);
        define_type(&self.symbols, &mut intrinsic_scope, "f64", Type::F64);
        define_type(&self.symbols, &mut intrinsic_scope, "bool", Type::Boolean);
        define_type(&self.symbols, &mut intrinsic_scope, "String", Type::String);

        let mut root_scope = Scope::new(Some(&intrinsic_scope));
        pass::build_module_decls(&self.symbols, &mut root_scope, &mut self.decls, ast)?;
        self.resolve_imports().await?;
        pass::build_module_exprs(&self.symbols, &mut root_scope, &mut self.decls, ast)?;
        pass::gen_module(self)?;
        Ok(())
    }

    /// Load and resolve imported symbols from other modules.
    async fn resolve_imports(&mut self) -> Result<(), CompilationError> {
        // TODO: Implement
        Ok(())
    }

    pub(crate) fn report_error(&self, err: &CompilationError) {
        let location = err.location();
        let mut line_ct = 1;
        let mut offset = 0;
        for line in self.src.lines() {
            let end_offset = offset + line.len();
            let token_end = location.end().min(end_offset);
            if offset <= location.start() && location.start() < end_offset {
                eprintln!(
                    "{}:{}:{} {}",
                    self.path,
                    line_ct,
                    location.start() - offset + 1,
                    err
                );
                eprintln!("{}", line);
                eprintln!(
                    "{}{}",
                    " ".repeat(location.start() - offset),
                    "^".repeat(token_end - location.start())
                );
                return;
            }
            line_ct += 1;
            offset = end_offset + 1;
        }

        eprintln!("{}:{:?}:{}", self.path, location.start(), err);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{self, FloatSuffix, IntegerSuffix},
        compiler::CompilationUnit,
        decl::{InternedSymbols, LocalDecl, Scope},
        oper,
        parser::saga_parser,
        pass::{self, assign_types},
        Type,
    };
    use futures_lite::future;

    #[test]
    fn parse_integer() {
        let arena = bumpalo::Bump::new();
        let symbols = InternedSymbols::new();
        let mut unit = CompilationUnit::new("--str--", "20");
        let node = saga_parser::expr(unit.src, &arena, &symbols).unwrap();
        assert!(matches!(
            node.kind,
            ast::NodeKind::LitInt(20, IntegerSuffix::Unsized)
        ));
        let mut inference: pass::TypeInference = Default::default();
        let mut root_scope = Scope::new(None);
        let mut locals = Vec::<LocalDecl>::new();
        let expr = pass::build_exprs(
            node,
            &unit.symbols,
            &mut root_scope,
            &mut unit.decls.functions,
            &mut locals,
            &mut inference,
        )
        .unwrap();
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
        let symbols = InternedSymbols::new();
        let mut unit = CompilationUnit::new("--str--", "20.0");
        let node = saga_parser::expr(unit.src, &arena, &symbols).unwrap();
        assert!(matches!(
            node.kind,
            ast::NodeKind::LitFloat(20.0, FloatSuffix::F32)
        ));
        let mut inference: pass::TypeInference = Default::default();
        let mut root_scope = Scope::new(None);
        let mut locals = Vec::<LocalDecl>::new();
        let expr = pass::build_exprs(
            node,
            &unit.symbols,
            &mut root_scope,
            &mut unit.decls.functions,
            &mut locals,
            &mut inference,
        )
        .unwrap();
        assert_eq!(expr.to_string(), "20.0");
        inference.solve_constraints().unwrap();
    }

    #[test]
    fn parse_binop_add() {
        let arena = bumpalo::Bump::new();
        let symbols = InternedSymbols::new();
        let mut unit = CompilationUnit::new("--str--", "20.0 + 10.0");
        let node = saga_parser::expr(unit.src, &arena, &symbols).unwrap();
        match &node.kind {
            ast::NodeKind::BinaryExpr { op, lhs, rhs } => {
                assert_eq!(*op, oper::BinaryOp::Add);
                assert!(matches!(
                    lhs.kind,
                    ast::NodeKind::LitFloat(20.0, FloatSuffix::F32)
                ));
                assert!(matches!(
                    rhs.kind,
                    ast::NodeKind::LitFloat(10.0, FloatSuffix::F32)
                ));
            }
            _ => panic!(),
        }
        let mut inference: pass::TypeInference = Default::default();
        let mut root_scope = Scope::new(None);
        let mut locals = Vec::<LocalDecl>::new();
        let mut expr = pass::build_exprs(
            node,
            &unit.symbols,
            &mut root_scope,
            &mut unit.decls.functions,
            &mut locals,
            &mut inference,
        )
        .unwrap();
        assert_eq!(expr.to_string(), "20.0 + 10.0");
        inference.solve_constraints().unwrap();
        assign_types(&mut expr, &inference).unwrap();
        assert_eq!(expr.typ, Type::F32);
    }

    #[test]
    fn parse_binop_prec() {
        let arena = bumpalo::Bump::new();
        let symbols = InternedSymbols::new();
        let mut unit = CompilationUnit::new("--str--", "20.0 + 10.0 * 0");
        let node = saga_parser::expr(unit.src, &arena, &symbols).unwrap();
        match &node.kind {
            ast::NodeKind::BinaryExpr { op, lhs, rhs } => {
                assert_eq!(*op, oper::BinaryOp::Add);
                assert!(matches!(
                    lhs.kind,
                    ast::NodeKind::LitFloat(20.0, FloatSuffix::F32)
                ));
                match &rhs.kind {
                    ast::NodeKind::BinaryExpr { op, lhs, rhs } => {
                        assert_eq!(*op, oper::BinaryOp::Mul);
                        assert!(matches!(
                            lhs.kind,
                            ast::NodeKind::LitFloat(10.0, FloatSuffix::F32)
                        ));
                        assert!(matches!(
                            rhs.kind,
                            ast::NodeKind::LitInt(0, IntegerSuffix::Unsized)
                        ));
                    }
                    _ => panic!(),
                }
                // assert!(matches!(rhs.value, ast::NodeValue::ConstF64(10.0)));
            }
            _ => panic!(),
        }
        let mut inference: pass::TypeInference = Default::default();
        let mut root_scope = Scope::new(None);
        let mut locals = Vec::<LocalDecl>::new();
        let expr = pass::build_exprs(
            node,
            &unit.symbols,
            &mut root_scope,
            &mut unit.decls.functions,
            &mut locals,
            &mut inference,
        )
        .unwrap();
        assert_eq!(expr.to_string(), "20.0 + 10.0 * 0");
        let err = inference.solve_constraints().unwrap_err();
        assert_eq!(err.to_string(), "Cannot assign type i32 to f32");
    }

    #[test]
    fn parse_module() {
        let arena = bumpalo::Bump::new();
        let symbols = InternedSymbols::new();
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
    }

    #[test]
    fn test_compiler() {
        let mut unit = CompilationUnit::new("--str--", "fn test() -> i32 { 1 + 2 }");
        future::block_on(unit.compile()).unwrap();

        // (type $tup (struct i64 i64 i32))
    }
}
