use super::ast::{ASTDecl, ASTFunctionParam, ASTNode, FloatSuffix, IntegerSuffix, NodeKind};
use super::location::TokenLocation;
use super::oper::{BinaryOp, UnaryOp};
use crate::sema::decl::DeclVisibility;
use bumpalo::Bump;

fn binary<'a>(
    arena: &'a Bump,
    op: BinaryOp,
    lhs: &'a ASTNode<'a>,
    rhs: &'a ASTNode<'a>,
) -> &'a ASTNode<'a> {
    let location = lhs.location.union(rhs.location);
    arena.alloc(ASTNode::new(
        location,
        NodeKind::BinaryExpr { op, lhs, rhs },
    ))
}

fn assign_op<'a>(
    arena: &'a Bump,
    op: BinaryOp,
    lhs: &'a ASTNode<'a>,
    rhs: &'a ASTNode<'a>,
) -> &'a ASTNode<'a> {
    let location = lhs.location.union(rhs.location);
    arena.alloc(ASTNode::new(location, NodeKind::AssignOp { op, lhs, rhs }))
}

fn unary<'a>(arena: &'a Bump, op: UnaryOp, start: usize, arg: &'a ASTNode<'a>) -> &'a ASTNode<'a> {
    let location = arg.location.union((start, start + 1));
    arena.alloc(ASTNode::new(location, NodeKind::UnaryExpr { op, arg }))
}

peg::parser! {
    pub(crate) grammar formula_parser<'a>(arena: &'a Bump) for str {
        rule ws() = quiet!{[' ' | '\n' | '\r' | '\t']}
        rule line_comment() = quiet!{("//" [^'\n']*)}
        rule _() = quiet!{(ws() / line_comment())*}

        rule digits() = n:$(['0'..='9']['0'..='9' | '_']*) { }
        rule name() -> &'a str =
            n:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*)
            {
                arena.alloc_str(n)
            }

        rule lit_float() -> &'a ASTNode<'a> =
            start:position!()
            n:$(
                digits() &(['.' | 'e' | 'f'])
                ("." ['0'..='9' | '_']*)?
                ("e" ['+' | '-']? digits())?
            )
            s:("f32" { FloatSuffix::F32 } / "f64" { FloatSuffix::F64 })?
            end:position!()
        {?
            let value = n.replace('_', "").parse::<f64>().map_err(|_| "valid float literal")?;
            Ok(arena.alloc(ASTNode::new(
                (start, end),
                NodeKind::LitFloat(value, s.unwrap_or(FloatSuffix::F32)),
            )))
        }

        rule lit_int() -> &'a ASTNode<'a> =
            start:position!()
            n:$digits()
            s:("i32" { IntegerSuffix::I32 } / "i64" { IntegerSuffix::I64 })?
            end:position!()
        {?
            let value = n.replace('_', "").parse::<i64>().map_err(|_| "valid integer literal")?;
            Ok(arena.alloc(ASTNode::new(
                (start, end),
                NodeKind::LitInt(value, s.unwrap_or(IntegerSuffix::Unsized)),
            )))
        }

        rule index() -> usize = n:$digits()
        {?
            n.replace('_', "").parse::<usize>().map_err(|_| "valid tuple index")
        }

        rule unicode_escape() -> char =
            ['u'] ['{'] d:$(['0'..='9' | 'a'..='f' | 'A'..='F']*<1,6>) ['}']
        {?
            u32::from_str_radix(d, 16)
                .ok()
                .and_then(core::char::from_u32)
                .ok_or("valid unicode escape")
        }

        rule lit_string() -> &'a ASTNode<'a> =
            start:position!()
            ['"']
            s:(
                (['\\'] ch:(
                    ['n'] { '\n' }
                    / ['r'] { '\r' }
                    / ['t'] { '\t' }
                    / ['"'] { '\"' }
                    / ['\''] { '\'' }
                    / ['\\'] { '\\' }
                    / ['0'] { '\0' }
                    / unicode_escape()
                ) { ch })
                / [^'"' | '\\']
                / expected!("string char")
            )*
            ['"']
            end:position!()
        {
            let value: String = s.into_iter().collect();
            arena.alloc(ASTNode::new((start, end), NodeKind::LitString(arena.alloc_str(&value))))
        }

        rule lit_bool() -> &'a ASTNode<'a> =
            start:position!()
            value:( "true" { true } / "false" { false } )
            end:position!()
        {
            arena.alloc(ASTNode::new((start, end), NodeKind::LitBool(value)))
        }

        rule ident() -> &'a ASTNode<'a> =
            start:position!()
            n:name()
            end:position!()
        {
            arena.alloc(ASTNode::new((start, end), NodeKind::Ident(n)))
        }

        rule primary() -> &'a ASTNode<'a> =
        e:(
            lit_float()
            / lit_int()
            / lit_string()
            / lit_bool()
            / qname()
            / "(" _ e:expr() _ ")" { e }
            / expected!("expression")
        ) { e }

        rule binop() -> &'a ASTNode<'a> = precedence!{
            lhs:(@) _ "=" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::Assign { lhs, rhs }))
            }
            lhs:(@) _ "+=" _ rhs:@ { assign_op(arena, BinaryOp::Add, lhs, rhs) }
            lhs:(@) _ "-=" _ rhs:@ { assign_op(arena, BinaryOp::Sub, lhs, rhs) }
            lhs:(@) _ "*=" _ rhs:@ { assign_op(arena, BinaryOp::Mul, lhs, rhs) }
            lhs:(@) _ "/=" _ rhs:@ { assign_op(arena, BinaryOp::Div, lhs, rhs) }
            lhs:(@) _ "%=" _ rhs:@ { assign_op(arena, BinaryOp::Mod, lhs, rhs) }
            lhs:(@) _ "|=" _ rhs:@ { assign_op(arena, BinaryOp::BitOr, lhs, rhs) }
            lhs:(@) _ "&=" _ rhs:@ { assign_op(arena, BinaryOp::BitAnd, lhs, rhs) }
            lhs:(@) _ "^=" _ rhs:@ { assign_op(arena, BinaryOp::BitXor, lhs, rhs) }
            lhs:(@) _ "<<=" _ rhs:@ { assign_op(arena, BinaryOp::Shl, lhs, rhs) }
            lhs:(@) _ ">>=" _ rhs:@ { assign_op(arena, BinaryOp::Shr, lhs, rhs) }
            --
            lhs:(@) _ "||" _ rhs:@ { binary(arena, BinaryOp::LogOr, lhs, rhs) }
            --
            lhs:(@) _ "&&" _ rhs:@ { binary(arena, BinaryOp::LogAnd, lhs, rhs) }
            --
            lhs:(@) _ "==" _ rhs:@ { binary(arena, BinaryOp::Eq, lhs, rhs) }
            lhs:(@) _ "!=" _ rhs:@ { binary(arena, BinaryOp::Ne, lhs, rhs) }
            lhs:(@) _ ">=" _ rhs:@ { binary(arena, BinaryOp::Ge, lhs, rhs) }
            lhs:(@) _ "<=" _ rhs:@ { binary(arena, BinaryOp::Le, lhs, rhs) }
            lhs:(@) _ ">" !['>' | '='] _ rhs:@ { binary(arena, BinaryOp::Gt, lhs, rhs) }
            lhs:(@) _ "<" !['<' | '='] _ rhs:@ { binary(arena, BinaryOp::Lt, lhs, rhs) }
            --
            lhs:(@) _ "|" !['|' | '='] _ rhs:@ { binary(arena, BinaryOp::BitOr, lhs, rhs) }
            --
            lhs:(@) _ "^" !['='] _ rhs:@ { binary(arena, BinaryOp::BitXor, lhs, rhs) }
            --
            lhs:(@) _ "&" !['&' | '='] _ rhs:@ { binary(arena, BinaryOp::BitAnd, lhs, rhs) }
            --
            lhs:(@) _ "<<" !['='] _ rhs:@ { binary(arena, BinaryOp::Shl, lhs, rhs) }
            lhs:(@) _ ">>" !['='] _ rhs:@ { binary(arena, BinaryOp::Shr, lhs, rhs) }
            --
            lhs:(@) _ "+" !['='] _ rhs:@ { binary(arena, BinaryOp::Add, lhs, rhs) }
            lhs:(@) _ "-" !['='] _ rhs:@ { binary(arena, BinaryOp::Sub, lhs, rhs) }
            --
            lhs:(@) _ "*" !['='] _ rhs:@ { binary(arena, BinaryOp::Mul, lhs, rhs) }
            lhs:(@) _ "/" !['='] _ rhs:@ { binary(arena, BinaryOp::Div, lhs, rhs) }
            lhs:(@) _ "%" !['='] _ rhs:@ { binary(arena, BinaryOp::Mod, lhs, rhs) }
            --
            arg:(@) _ "as" _ typ:@ {
                let location = arg.location.union(typ.location);
                arena.alloc(ASTNode::new(location, NodeKind::Cast { arg, typ }))
            }
            --
            start:position!() "-" arg:(@) { unary(arena, UnaryOp::Neg, start, arg) }
            start:position!() "!" arg:(@) { unary(arena, UnaryOp::Not, start, arg) }
            start:position!() "~" arg:(@) { unary(arena, UnaryOp::BitNot, start, arg) }
            func:(@) _ start:position!() "(" _ args:(a: expr() ** (_ "," _) { a }) _ ")" end:position!() {
                let location = func.location.union((start, end));
                arena.alloc(
                    ASTNode::new(location, NodeKind::Call(func, arena.alloc_slice_copy(args.as_slice()))))
            }
            base:(@) _ "." _ start:position!() field:name() end:position!() {
                let location = base.location.union((start, end));
                arena.alloc(ASTNode::new(location, NodeKind::FieldName(base, field)))
            }
            base:(@) _ "." _ start:position!() index() end:position!() {
                let location = base.location.union((start, end));
                arena.alloc(ASTNode::new(location, NodeKind::FieldIndex))
            }
            p:primary() { p }
        }

        pub rule expr() -> &'a ASTNode<'a> = l:binop() { l } / expected!("expression")

        rule empty_stmt() -> &'a ASTNode<'a> = start:position!() ";" {
            arena.alloc(ASTNode::new((start, start + 1), NodeKind::Empty))
        }

        rule qname() -> &'a ASTNode<'a> =
            start:position!()
            n:(ident() ++ (_ "::" _))
            end:position!()
            {
                if n.len() == 1 {
                    n[0]
                } else {
                    let names = arena.alloc_slice_copy(n.as_slice());
                    arena.alloc(ASTNode::new((start, end), NodeKind::QName(names)))
                }
            }

        rule type_name() -> &'a ASTNode<'a> = n:qname() { n }

        rule array_type() -> &'a ASTNode<'a> =
            start:position!()
            "[" _ type_expr() _ "]"
            end:position!()
        {
            arena.alloc(ASTNode::new((start, end), NodeKind::ArrayType))
        }

        pub rule type_expr() -> &'a ASTNode<'a> = type_name() / array_type()

        pub rule var_decl() -> &'a ASTNode<'a> =
            start:position!()
            is_const:("let" { false } / "const" { true })
            _ id:name()
            ty:(_ ":" _ ty:type_expr() { ty })?
            init: (_ "=" _ e:expr() { e })?
            _ ";"
            end:position!() {
                let location = (start, end);
                arena.alloc(ASTNode::new(location, NodeKind::Decl(arena.alloc(ASTDecl::Let {
                    name: id,
                    visibility: DeclVisibility::Private,
                    typ: ty,
                    value: init,
                    is_const
                }))))
            }

        pub rule condition() -> &'a ASTNode<'a> = e:expr() { e } / expected!("condition")

        pub(crate) rule if_stmt() -> &'a ASTNode<'a> =
            start:position!()
            "if"
            _ condition:condition()
            _ then_block:block()
            else_block: (
                _ "else"
                _ s:(
                    s:block() { s }
                    / s:if_stmt() { s }
                ) { s }
            )?
            end:position!()
            {
                let location = (start, end);
                arena.alloc(ASTNode::new(location, NodeKind::If {
                    condition, then_block, else_block
                }))
            }

        rule stmt() -> &'a ASTNode<'a> =
            _
            s:(
                s0:empty_stmt() { s0 }
                / v:var_decl() { v }
                / i:if_stmt() !(_ "}") [_] { i }
                / b:block() !(_ "}") [_] { b }
                / s:expr() (";" / expected!("semicolon")) { s }
            )
            { s } / expected!("statement")

        rule block_contents() -> (Vec<&'a ASTNode<'a>>, Option<&'a ASTNode<'a>>)  =
            l:(stmt()*)
            f:(
                _ e: if_stmt() { e }
                / _ e: block() { e }
                / _ e: expr() { e }
            )?
            _
        {
            (l, f)
        }

        pub(crate) rule block() -> &'a ASTNode<'a> =
            start:position!()
            "{"
            c:block_contents()
            "}"
            end:position!()
        {
            let location = (start, end);
            let stmts = arena.alloc_slice_copy(c.0.as_slice());
            arena.alloc(ASTNode::new(location, NodeKind::Block(stmts, c.1)))
        }

        rule visibility() -> DeclVisibility =
            "pub" { DeclVisibility::Public } / { DeclVisibility::Private }

        rule param_decl() -> &'a ASTFunctionParam<'a> =
            start:position!()
            id:name() _ ":" _ ty:type_expr()
            end:position!()
        {
            arena.alloc(ASTFunctionParam {
                location: TokenLocation::new(start, end),
                name: id,
                typ: ty,
            })
        }

        pub rule param_list() -> &'a[&'a ASTFunctionParam<'a>] =
            "(" _ params:(param_decl() ** (_ "," _)) _ ("," _)? ")"
            { arena.alloc_slice_copy(params.as_slice()) }

        rule func_return() -> &'a ASTNode<'a> = "->" _ t:type_expr() { t }
        rule func_defn() -> &'a ASTNode<'a> =
            start:position!()
            vis:visibility() _
            "fn" _ id:name() _ p:param_list() _ ret:func_return()? _
            body:block()
            end:position!()
        {
            let location = (start, end);
            arena.alloc(ASTNode::new(location, NodeKind::Decl(arena.alloc(ASTDecl::Function {
                name: id,
                visibility: vis,
                params: p,
                ret,
                body,
            }))))
        }

        rule struct_field() = name() _ ":" _ type_expr()

        rule struct_defn() -> &'a ASTNode<'a> =
            start:position!()
            visibility() _
            "struct" _ name() _
            "{" _ (struct_field() ** (_ "," _)) _ ("," _)? "}"
            end:position!()
        {
            let decl = arena.alloc(ASTDecl::Struct);
            arena.alloc(ASTNode::new((start, end), NodeKind::Decl(decl)))
        }

        rule import_items() -> Vec<&'a str> = "{" _ n:(name() ** (_ "," _)) _ ("," _)? "}" { n }

        rule import_decl() -> &'a ASTNode<'a> =
            start:position!()
            "import" _ path:(name() ++ (_ "::" _))
            items:(_ "::" _ i:import_items() { i })?
            _ ";"
            end:position!()
        {
            let path = arena.alloc_slice_copy(path.as_slice());
            let items = arena.alloc_slice_copy(items.unwrap_or_default().as_slice());
            arena.alloc(ASTNode::new((start, end), NodeKind::Import(path, items)))
        }

        pub rule decl() -> &'a ASTNode<'a>
            = i:import_decl() { i }
            / f:func_defn() { f }
            / s:struct_defn() { s }
            / v:var_decl() { v }
            / expected!("declaration")

        pub rule formula() -> &'a ASTNode<'a> =
            start:position!()
            c:block_contents()
            end:position!()
        {
            let location = (start, end);
            let stmts = arena.alloc_slice_copy(c.0.as_slice());
            arena.alloc(ASTNode::new(location, NodeKind::Block(stmts, c.1)))
        }

        pub rule module() -> &'a ASTNode<'a> =
            start:position!() _ d:(d:decl() _ { d })* end:position!()
        {
            let decls = arena.alloc_slice_copy(d.as_slice());
            arena.alloc(ASTNode::new((start, end), NodeKind::Module(decls)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;

    #[test]
    pub fn literal_int() {
        let arena = Bump::new();
        let ast = formula_parser::expr("1", &arena);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitInt(1, IntegerSuffix::Unsized),
                ..
            })
        ));
    }

    #[test]
    pub fn literal_int_i32() {
        let arena = Bump::new();
        let ast = formula_parser::expr("1i32", &arena);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitInt(1, IntegerSuffix::I32),
                ..
            })
        ));
    }

    #[test]
    pub fn literal_int_i64() {
        let arena = Bump::new();
        let ast = formula_parser::expr("1_000i64", &arena);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitInt(1000, IntegerSuffix::I64),
                ..
            })
        ));
    }

    #[test]
    pub fn literal_float() {
        let arena = Bump::new();
        let cases = [
            ("1.0", 1.0, FloatSuffix::F32),
            ("1.", 1.0, FloatSuffix::F32),
            ("1.0e10", 1.0e10, FloatSuffix::F32),
            ("1.0f32", 1.0, FloatSuffix::F32),
            ("1.0f64", 1.0, FloatSuffix::F64),
        ];
        for (src, value, suffix) in cases {
            let ast = formula_parser::expr(src, &arena).unwrap();
            let NodeKind::LitFloat(v, s) = ast.kind else {
                panic!("{src} is not a float literal");
            };
            assert_eq!(v, value, "{src}");
            assert_eq!(s, suffix, "{src}");
        }
    }

    #[test]
    pub fn literal_string() {
        let arena = Bump::new();
        let cases = [
            ("\"hello\"", "hello"),
            ("\"hello\\n\"", "hello\n"),
            ("\"hello\\\"\"", "hello\""),
            ("\"hello\u{25ff}\"", "hello◿"),
            ("\"\\u{25ff}\"", "◿"),
        ];
        for (src, value) in cases {
            let ast = formula_parser::expr(src, &arena).unwrap();
            let NodeKind::LitString(s) = ast.kind else {
                panic!("{src} is not a string literal");
            };
            assert_eq!(s, value);
        }
    }

    #[test]
    pub fn invalid_unicode_escape() {
        let arena = Bump::new();
        assert!(formula_parser::expr("\"\\u{d800}\"", &arena).is_err());
        assert!(formula_parser::expr("\"\\u{ffffff}\"", &arena).is_err());
    }

    #[test]
    pub fn binop_add() {
        let arena = Bump::new();
        let ast = formula_parser::expr("1.0 + 5", &arena);
        match ast {
            Ok(ASTNode {
                kind: NodeKind::BinaryExpr { op, lhs, rhs },
                location,
                ..
            }) => {
                assert_eq!(location.start(), 0);
                assert_eq!(location.end(), 7);
                assert_eq!(*op, BinaryOp::Add);
                assert!(matches!(
                    lhs.kind,
                    NodeKind::LitFloat(1.0, FloatSuffix::F32)
                ));
                assert!(matches!(
                    rhs.kind,
                    NodeKind::LitInt(5, IntegerSuffix::Unsized)
                ));
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn binop_shift_and_compare() {
        let arena = Bump::new();
        let ast = formula_parser::expr("1 << 2 < 8", &arena).unwrap();
        let NodeKind::BinaryExpr { op, lhs, .. } = ast.kind else {
            panic!()
        };
        assert_eq!(op, BinaryOp::Lt);
        assert!(matches!(
            lhs.kind,
            NodeKind::BinaryExpr {
                op: BinaryOp::Shl,
                ..
            }
        ));
    }

    #[test]
    pub fn unary_location() {
        let arena = Bump::new();
        let ast = formula_parser::expr("!x", &arena).unwrap();
        assert_eq!(ast.location.start(), 0);
        assert_eq!(ast.location.end(), 2);
    }

    #[test]
    pub fn binop_err() {
        let arena = Bump::new();
        let ast = formula_parser::expr("1.0 + +", &arena).unwrap_err();
        assert_eq!(ast.location.offset, 6);
    }

    #[test]
    pub fn param_list() {
        let arena = Bump::new();
        let ast = formula_parser::param_list("()", &arena).unwrap();
        assert_eq!(ast.len(), 0);

        let ast = formula_parser::param_list("(a: bool)", &arena).unwrap();
        assert_eq!(ast.len(), 1);

        let ast = formula_parser::param_list("(a: bool,)", &arena).unwrap();
        assert_eq!(ast.len(), 1);

        let ast = formula_parser::param_list("(a: bool, b: bool)", &arena).unwrap();
        assert_eq!(ast.len(), 2);

        let ast = formula_parser::param_list("(a: bool, b: bool,)", &arena).unwrap();
        assert_eq!(ast.len(), 2);
    }

    #[test]
    pub fn block() {
        let arena = Bump::new();
        let cases = [
            ("{}", 0, false),
            ("{ 10 }", 0, true),
            ("{ 10; }", 1, false),
            ("{ 10; 10 }", 1, true),
            ("{ if true { 0 } }", 0, true),
            ("{ if true { 0 } 10 }", 1, true),
            ("{ { 0 } }", 0, true),
            ("{ { 1 } { 0 } }", 1, true),
        ];
        for (src, num_stmts, has_result) in cases {
            let ast = formula_parser::block(src, &arena).unwrap();
            let NodeKind::Block(stmts, result) = ast.kind else {
                panic!("{src} is not a block")
            };
            assert_eq!(stmts.len(), num_stmts, "{src}");
            assert_eq!(result.is_some(), has_result, "{src}");
        }

        let ast = formula_parser::formula("{ 0 }", &arena).unwrap();
        let NodeKind::Block(stmts, result) = ast.kind else {
            panic!()
        };
        assert_eq!(stmts.len(), 0);
        assert!(result.is_some());
    }

    #[test]
    pub fn if_stmt() {
        let arena = Bump::new();
        let ast = formula_parser::if_stmt("if x > 0 { 1.0 }", &arena);
        match ast {
            Ok(ASTNode {
                kind:
                    NodeKind::If {
                        condition: test,
                        then_block,
                        else_block,
                    },
                location,
                ..
            }) => {
                assert_eq!(location.start(), 0);
                assert_eq!(location.end(), 16);
                assert!(matches!(test.kind, NodeKind::BinaryExpr { .. }));
                assert!(matches!(then_block.kind, NodeKind::Block(_, _)));
                assert!(else_block.is_none());
            }
            Err(err) => panic!("Parse error: {err} at {}", err.location),
            _ => panic!("Unexpected AST: {ast:?}"),
        }
    }

    #[test]
    pub fn if_else_stmt() {
        let arena = Bump::new();
        let ast = formula_parser::if_stmt("if x > 0 { 1.0 } else { 2.0 }", &arena);
        match ast {
            Ok(ASTNode {
                kind:
                    NodeKind::If {
                        condition: test,
                        then_block,
                        else_block,
                    },
                location,
                ..
            }) => {
                assert_eq!(location.start(), 0);
                assert_eq!(location.end(), 29);
                assert!(matches!(test.kind, NodeKind::BinaryExpr { .. }));
                assert!(matches!(then_block.kind, NodeKind::Block(_, _)));
                assert!(matches!(else_block.unwrap().kind, NodeKind::Block(_, _)));
            }
            Err(err) => panic!("Parse error: {err} at {}", err.location),
            _ => panic!("Unexpected AST: {ast:?}"),
        }
    }

    #[test]
    pub fn import_decl() {
        let arena = Bump::new();
        let ast =
            formula_parser::decl("import package::actors::util::{near, flank};", &arena).unwrap();
        let NodeKind::Import(path, items) = ast.kind else {
            panic!()
        };
        assert_eq!(path, &["package", "actors", "util"]);
        assert_eq!(items, &["near", "flank"]);

        let ast = formula_parser::decl("import super::util::near;", &arena).unwrap();
        let NodeKind::Import(path, items) = ast.kind else {
            panic!()
        };
        assert_eq!(path, &["super", "util", "near"]);
        assert!(items.is_empty());
    }
}
