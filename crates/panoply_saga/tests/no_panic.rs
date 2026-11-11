//! A mod's script is untrusted input: whatever it says, the compiler reports an error rather
//! than panicking.

use std::panic::{AssertUnwindSafe, catch_unwind};

use futures_lite::future;
use panoply_saga::{
    CompilationError, ExprType, HostState, NoImports, ScriptModule, compile_formula, compile_module,
};

fn formula(src: &str) -> Result<ScriptModule, CompilationError> {
    let host = HostState::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        future::block_on(compile_formula(
            "--str--",
            src,
            &host,
            &NoImports,
            ExprType::I32,
        ))
    }));
    result.unwrap_or_else(|_| panic!("compiling the formula {src:?} panicked"))
}

fn module(src: &str) -> Result<ScriptModule, CompilationError> {
    let host = HostState::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        future::block_on(compile_module("--str--", src, &host, &NoImports))
    }));
    result.unwrap_or_else(|_| panic!("compiling the module {src:?} panicked"))
}

#[test]
fn unsupported_features_are_errors() {
    let formulas = [
        "let v = 1; v.0",
        "let a: [i32] = 1; 0",
        "a::b::c",
        "let a: Entity::Kind = 1; 0",
    ];
    for src in formulas {
        match formula(src) {
            Err(CompilationError::Unsupported(..)) => {}
            other => panic!("{src:?}: expected Unsupported, got {other:?}"),
        }
    }
    let modules = [
        "import package::a::b;",
        "import super::util::{near, flank};",
        "struct S { x: i32 }",
        "let x = 1;",
        "const x = 1;",
        "fn f() -> i32 { 1 } fn g() -> i32 { let h = f; 0 }",
    ];
    for src in modules {
        match module(src) {
            Err(CompilationError::Unsupported(..)) => {}
            other => panic!("{src:?}: expected Unsupported, got {other:?}"),
        }
    }
}

#[test]
fn malformed_input_is_an_error() {
    let formulas = [
        "",
        "{",
        "}",
        "1 +",
        "1 + true",
        "\"\\u{d800}\"",
        "foo",
        "self",
        "i32",
        "Entity",
        "if 1 { 2 } else { 3 }",
        "if true { 1 }",
        "-true",
        "!1",
        "~1.0",
        "1 << 1.0",
        "true < false",
        "1.0 & 2.0",
        "\"a\" + \"b\"",
        "\"abc\".len(1)",
        "\"abc\".nope()",
        "1.len()",
        "1(2)",
        "x = 1; 0",
        "1 = 2; 0",
        "99999999999i32",
        "99999999999999999999",
        "let x; x",
        "{ let x = x; x }",
        "let x: Nope = 1; 0",
        "1 as Entity",
        "fn f() {} 1",
        "@",
        "1 +* 2",
        "\"unterminated",
        "(1",
        "1 += 2; 0",
        "let x = true; x += 1; 0",
    ];
    for src in formulas {
        assert!(formula(src).is_err(), "{src:?} compiled");
    }
    let modules = [
        "fn",
        "fn f(",
        "fn f() -> i32",
        "fn f() -> Nope { 1 }",
        "fn f(a: Nope) {}",
        "fn f() -> i32 { 1.0 }",
        "fn f() { g() }",
        "fn f() -> i32 { f(1) }",
        "native fn f() -> i32;",
        "pub",
        "1",
    ];
    for src in modules {
        assert!(module(src).is_err(), "{src:?} compiled");
    }
}

#[test]
fn deep_nesting_is_an_error() {
    let parens = format!("{}1{}", "(".repeat(10_000), ")".repeat(10_000));
    assert!(formula(&parens).is_err());
    let blocks = format!("{}1{}", "{".repeat(10_000), "}".repeat(10_000));
    assert!(formula(&blocks).is_err());
    let nots = format!("{}true as i32", "!".repeat(10_000));
    assert!(formula(&nots).is_err());
    let chain = format!("1{}", " + 1".repeat(100_000));
    assert!(formula(&chain).is_err());
}
