//! Compiling a module or a formula, and the errors it reports.

use std::sync::Arc;

use bumpalo::Bump;
use peg::{error::ParseError, str::LineCol};
use smol_str::SmolStr;
use thiserror::Error;

use crate::{
    ScriptModule,
    codegen::emit::gen_code,
    host::HostState,
    sema::{
        build::{build_formula_exprs, build_module_exprs},
        types::ExprType,
    },
    syntax::{
        location::TokenLocation,
        oper::{BinaryOp, UnaryOp},
        parser::formula_parser,
    },
};

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("Expression expected")]
    ExpectExpression(TokenLocation),
    #[error("Declaration expected")]
    ExpectDeclaration(TokenLocation),
    #[error("Statement expected")]
    ExpectStatement(TokenLocation),
    #[error("Semicolon expected")]
    ExpectSemi(TokenLocation),
    #[error("Expected {1}")]
    Expected(TokenLocation, SmolStr),
    #[error("Cannot assign type {2} to {1}")]
    MismatchedTypes(TokenLocation, ExprType, ExprType),
    #[error("Cannot convert type from {2} to {1}")]
    InvalidCast(TokenLocation, ExprType, ExprType),
    #[error("Call expression requires function")]
    NotCallable(TokenLocation),
    #[error("Function expects {1} arguments, got {2}")]
    IncorrectNumberOfArguments(TokenLocation, usize, usize),
    #[error("Recursive type: {1}")]
    RecursiveType(TokenLocation, ExprType),
    #[error("Cannot infer the type of this expression")]
    UnresolvedType(TokenLocation),
    #[error("Unknown type: {1}")]
    UnknownType(TokenLocation, SmolStr),
    #[error("Unknown field: {1}.{2}")]
    UnknownField(TokenLocation, SmolStr, SmolStr),
    #[error("Type {1} does not have fields")]
    NoFields(TokenLocation, ExprType),
    #[error("'{1}' has no members")]
    NoMembers(TokenLocation, SmolStr),
    #[error("Can't find the name '{1}' in this scope")]
    UnknownSymbol(TokenLocation, String),
    #[error("'{1}' is a type, not a value")]
    TypeAsValue(TokenLocation, SmolStr),
    #[error("Invalid types for binary operator {1}: {2} and {3}")]
    InvalidBinaryOpType(TokenLocation, BinaryOp, ExprType, ExprType),
    #[error("Invalid type for unary operator {1:?}: {2}")]
    InvalidUnaryOpType(TokenLocation, UnaryOp, ExprType),
    #[error("Cannot assign to this expression")]
    InvalidAssignmentTarget(TokenLocation),
    #[error("Function redefinition: {1}")]
    FunctionRedefinition(TokenLocation, String),
    #[error("Name redefinition: {1}")]
    NameRedefinition(TokenLocation, String),
    #[error("Cannot import '{1}'")]
    UnresolvedImport(TokenLocation, String),
    #[error("Not supported yet: {1}")]
    Unsupported(TokenLocation, &'static str),
    /// A broken invariant in the compiler: a bug, reported rather than panicking so that it
    /// fails one script, not the game.
    #[error("Internal compiler error: {1}")]
    Internal(TokenLocation, &'static str),
}

impl CompilationError {
    pub fn location(&self) -> TokenLocation {
        use CompilationError::*;
        match self {
            ExpectExpression(loc)
            | ExpectDeclaration(loc)
            | ExpectStatement(loc)
            | ExpectSemi(loc)
            | Expected(loc, _)
            | MismatchedTypes(loc, _, _)
            | InvalidCast(loc, _, _)
            | NotCallable(loc)
            | IncorrectNumberOfArguments(loc, _, _)
            | RecursiveType(loc, _)
            | UnresolvedType(loc)
            | UnknownType(loc, _)
            | UnknownField(loc, _, _)
            | NoFields(loc, _)
            | NoMembers(loc, _)
            | UnknownSymbol(loc, _)
            | TypeAsValue(loc, _)
            | InvalidBinaryOpType(loc, _, _, _)
            | InvalidUnaryOpType(loc, _, _)
            | InvalidAssignmentTarget(loc)
            | FunctionRedefinition(loc, _)
            | NameRedefinition(loc, _)
            | UnresolvedImport(loc, _)
            | Unsupported(loc, _)
            | Internal(loc, _) => *loc,
        }
    }
}

/// Loads the modules a module imports.
pub trait ModuleResolver {
    /// The module an import names by `path`, compiled.
    fn resolve(
        &self,
        location: TokenLocation,
        path: &[&str],
    ) -> impl Future<Output = Result<Arc<ScriptModule>, CompilationError>> + Send;
}

/// A resolver for modules that import nothing: it refuses every import.
pub struct NoImports;

impl ModuleResolver for NoImports {
    async fn resolve(
        &self,
        location: TokenLocation,
        path: &[&str],
    ) -> Result<Arc<ScriptModule>, CompilationError> {
        Err(CompilationError::UnresolvedImport(
            location,
            path.join("::"),
        ))
    }
}

/// The error, with the line and its source token underlined.
pub fn format_error(path: &str, src: &str, err: &CompilationError) -> String {
    let location = err.location();
    let mut offset = 0;
    for (line_index, line) in src.split('\n').enumerate() {
        let end_offset = offset + line.len();
        if offset <= location.start() && location.start() <= end_offset {
            let column = location.start() - offset;
            let width = location
                .end()
                .min(end_offset)
                .saturating_sub(location.start())
                .max(1);
            return format!(
                "{path}:{}:{} {err}\n{line}\n{}{}",
                line_index + 1,
                column + 1,
                " ".repeat(column),
                "^".repeat(width)
            );
        }
        offset = end_offset + 1;
    }
    format!("{path}:{} {err}", location.start())
}

/// How deeply brackets and prefix operators may nest. The parser recurses on each, so without a
/// limit a deep enough script would overflow the stack, which aborts the process.
const MAX_NESTING: usize = 128;

/// Rejects source whose brackets or runs of prefix operators nest deeper than [`MAX_NESTING`],
/// before the parser recurses into them. String literals are skipped.
fn check_nesting(src: &str) -> Result<(), CompilationError> {
    let mut depth = 0usize;
    let mut prefix_run = 0usize;
    let mut chars = src.char_indices();
    while let Some((offset, c)) = chars.next() {
        let too_deep = || {
            CompilationError::Unsupported(
                TokenLocation::new(offset, offset + 1),
                "nesting this deep",
            )
        };
        match c {
            '"' => {
                // Skip to the closing quote, past escapes.
                while let Some((_, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
                prefix_run = 0;
            }
            '(' | '{' | '[' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return Err(too_deep());
                }
            }
            ')' | '}' | ']' => depth = depth.saturating_sub(1),
            '!' | '-' | '~' => {
                prefix_run += 1;
                if prefix_run > MAX_NESTING {
                    return Err(too_deep());
                }
            }
            c if c.is_whitespace() => {}
            _ => prefix_run = 0,
        }
    }
    Ok(())
}

fn transform_parse_error(err: ParseError<LineCol>) -> CompilationError {
    let location = TokenLocation::new(err.location.offset, err.location.offset + 1);
    for token in err.expected.tokens() {
        match token {
            "expression" => return CompilationError::ExpectExpression(location),
            "declaration" => return CompilationError::ExpectDeclaration(location),
            "statement" => return CompilationError::ExpectStatement(location),
            "semicolon" => return CompilationError::ExpectSemi(location),
            _ => {}
        }
    }
    CompilationError::Expected(location, err.expected.to_string().into())
}

/// Compile a module: a file of declarations.
pub async fn compile_module(
    path: &str,
    src: &str,
    host: &HostState,
    // TODO(PAN71): resolve the module's imports.
    _resolver: &impl ModuleResolver,
) -> Result<ScriptModule, CompilationError> {
    let mut module = ScriptModule {
        path: path.to_string(),
        ..Default::default()
    };
    check_nesting(src)?;
    let ast_arena = Bump::new();
    let ast = formula_parser::module(src, &ast_arena).map_err(transform_parse_error)?;
    let expr_arena = Bump::new();
    let exprs = build_module_exprs(host, &mut module, ast, &expr_arena)?;
    gen_code(&mut module, &exprs)?;
    Ok(module)
}

/// Compile a formula: statements and a result of type `result_type`, which run as the module's
/// [`ScriptModule::DEFAULT`] function.
pub async fn compile_formula(
    path: &str,
    src: &str,
    host: &HostState,
    _resolver: &impl ModuleResolver,
    result_type: ExprType,
) -> Result<ScriptModule, CompilationError> {
    let mut module = ScriptModule {
        path: path.to_string(),
        ..Default::default()
    };
    check_nesting(src)?;
    let ast_arena = Bump::new();
    let ast = formula_parser::formula(src, &ast_arena).map_err(transform_parse_error)?;
    let expr_arena = Bump::new();
    let exprs = build_formula_exprs(host, &mut module, ast, result_type, &expr_arena)?;
    gen_code(&mut module, &exprs)?;
    Ok(module)
}
