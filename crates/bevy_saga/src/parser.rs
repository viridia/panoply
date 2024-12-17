use crate::ast::{
    ASTNode, DeclKind, FloatSuffix, FunctionParam, IntegerSuffix, NodeKind, TypeKind,
};
use crate::decl;
use crate::oper::{BinaryOp, UnaryOp};
use bumpalo::Bump;

peg::parser! {
    pub grammar saga_parser<'a, 's>(arena: &'a Bump, symbols: &'s decl::InternedSymbols) for str {
        rule ws() = quiet!{[' ' | '\n' | '\t']}
        rule line_comment() = quiet!{("//" [^'\n']*)}
        rule _() = quiet!{(ws() / line_comment())*}

        rule digits() = n:$(['0'..='9']['0'..='9' | '_']*) { }
        rule name() -> decl::Symbol =
            n:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*)
            {
                symbols.intern(n)
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
            let value = n.parse::<f64>().map_err(|_| "invalid float literal")?;
            Ok(arena.alloc(ASTNode::new((start, end), NodeKind::LitFloat(value, s.unwrap_or(FloatSuffix::F32)))))
        }

        rule lit_int() -> &'a ASTNode<'a> =
            start:position!()
            n:$(digits())
            s:("i32" { IntegerSuffix::I32 } / "i64" { IntegerSuffix::I64 })?
            end:position!()
        {?
            // let location = TokenLocation::new(start, end);
            let value = n.parse::<i64>().map_err(|_| "invalid integer literal")?;
            Ok(arena.alloc(ASTNode::new((start, end), NodeKind::LitInt(value, s.unwrap_or(IntegerSuffix::Unsized)))))
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
                    / ['u'] ['{'] d:$(['0'..='9' | 'a'..='f' | 'A'..='F']*<1,6>) ['}'] {
                        let code = u32::from_str_radix(d, 16).unwrap();
                        core::char::from_u32(code).unwrap()
                    }
                ) { ch })
                / [^'"' | '\\']
                / expected!("string char")
            )*
            ['"']
            end:position!()
        {
            let mut value = String::new();
            for part in s {
                value.push(part);
            }
            arena.alloc(ASTNode::new((start, end), NodeKind::LitString(symbols.intern(&value))))
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
            / ident()
            / "(" _ e:expr() _ ")" { e }
            / expected!("expression")
        ) { e }

        rule binop() -> &'a ASTNode<'a> = precedence!{
            lhs:(@) _ "||" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::LogOr,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "&&" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::LogAnd,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "==" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Eq,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "!=" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Ne,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ ">" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Gt,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ ">=" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Ge,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "<" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Lt,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "<=" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Le,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "|" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::BitOr,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "^" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::BitXor,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "&" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::BitAnd,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "<<" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Shl,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ ">>" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Shr,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "+" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Add,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "-" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Sub,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "*" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Mul,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "/" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Div,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "%" _ rhs:@ {
                let location = lhs.location.union(rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Mod,
                    lhs,
                    rhs,
                }))
            }
            --
            arg:(@) _ "as" _ typ:@ {
                let location = arg.location.union(typ.location);
                arena.alloc(ASTNode::new(location, NodeKind::Cast {
                    arg,
                    typ,
                }))
            }
            --
            "-" arg:(@) {
                arena.alloc(ASTNode::new(arg.location, NodeKind::UnaryExpr {
                    op: UnaryOp::Neg,
                    arg,
                }))
            }
            "!" arg:(@) {
                arena.alloc(ASTNode::new(arg.location, NodeKind::UnaryExpr {
                    op: UnaryOp::Not,
                    arg,
                }))
            }
            "~" arg:(@) {
                arena.alloc(ASTNode::new(arg.location, NodeKind::UnaryExpr {
                    op: UnaryOp::BitNot,
                    arg,
                }))
            }
            func:(@) _ start:position!() "(" _ args:(a: expr() ** (_ "," _) { a }) _ ")" end:position!() {
                let location = func.location.union((start, end));
                arena.alloc(
                    ASTNode::new(location, NodeKind::Call(func, arena.alloc_slice_copy(args.as_slice()))))
            }
            base:(@) _ "." _ start:position!() field:name() end:position!() {
                let location = base.location.union((start, end));
                arena.alloc(
                    ASTNode::new(location, NodeKind::Field(base, field)))
            }
            p:primary() { p }
        }

        pub rule expr() -> &'a ASTNode<'a> = l:(binop()) { l } / expected!("expression")
        rule empty_stmt() -> &'a ASTNode<'a> = ";" _ {
            arena.alloc(ASTNode::new((0, 0), NodeKind::Empty))
        }

        rule type_name() -> &'a ASTNode<'a> =
            start:position!()
            n:(ident() ** (_ "::" _))
            end:position!()
            {
                if n.len() == 1 {
                    n[0]
                } else {
                    let names = arena.alloc_slice_copy(n.as_slice());
                    arena.alloc(ASTNode::new((start, end), NodeKind::QName(names)))
                }
            }

        rule array_type() -> &'a ASTNode<'a> =
            start:position!()
            "[" _ t:type_expr() _ "]"
            end:position!()
        {
            arena.alloc(ASTNode::new((start, end), NodeKind::Type(TypeKind::Array(t))))
        }

        pub rule type_expr() -> &'a ASTNode<'a> = type_name() / array_type()

        pub rule var_decl() -> &'a ASTNode<'a> =
            start:position!()
            is_const:("let" { false } / "const" { true })
            _ id:name()
            ty:(_ ":" _ ty:type_expr() { ty })?
            init: (_ "=" _ e:expr() { e })?
            _ ";" _
            end:position!() {
            let location = (start, end);
            arena.alloc(ASTNode::new(location, NodeKind::Decl(arena.alloc(DeclKind::Let {
                name: id,
                typ: ty,
                value: init,
                is_const
            }))))
        }

        rule stmt() -> &'a ASTNode<'a> =
            start:position!()
            s:(
                s0: empty_stmt() { s0 }
                / v: var_decl() { v }
                / s:(expr()) _ ";" _ { s }
            )
            end:position!()
            { s } / expected!("statement")

        rule block() -> &'a ASTNode<'a> =
            start:position!()
            "{" _
            l:(stmt()*)
            f:expr()?
            _ "}"
            end:position!()
        {
            let location = (start, end);
            let stmts = arena.alloc_slice_copy(l.as_slice());
            arena.alloc(ASTNode::new(location, NodeKind::Block(stmts, f)))
        }

        rule visiblity() -> decl::DeclVisibility =
            "pub" { decl::DeclVisibility::Public } / { decl::DeclVisibility::Private }

        rule param_decl() -> &'a FunctionParam<'a> =
            start:position!()
            id:name() _ ":" _ ty:type_expr()
            end:position!()
        {
            let location = (start, end);
            arena.alloc(FunctionParam {
                location: (start, end).into(),
                name: id,
                typ: ty,
            })
        }

        pub rule param_list() -> &'a[&'a FunctionParam<'a>] =
            "(" _
            params:(
                p0:param_decl() _
                p1:("," _ p: param_decl() _ { p })*
                ("," _)?
                {
                    let mut params = Vec::with_capacity(1 + p1.len());
                    params.push(p0);
                    params.extend(p1);
                    arena.alloc_slice_copy(params.as_slice())
                }
            )?
            ")"
            { params.unwrap_or(arena.alloc_slice_copy(&[])) }

        rule func_return() -> &'a ASTNode<'a> = "->" _ t:type_expr() { t }
        rule func_body() -> &'a ASTNode<'a> = t:block() { t }
        rule func_defn() -> &'a ASTNode<'a> =
            start:position!()
            vis:visiblity() _
            "fn" _ id:name() _ p:param_list() _ r:func_return()? _ b:func_body()
            end:position!()
        {
            let location = (start, end);
            arena.alloc(ASTNode::new(location, NodeKind::Decl(arena.alloc(DeclKind::Function {
                name: id,
                visibility: vis,
                params: p,
                ret: r.unwrap_or_else(|| arena.alloc(ASTNode::new((0, 0), NodeKind::Empty))),
                body: b,
            }))))
        }

        pub rule decl() -> &'a ASTNode<'a> = f:func_defn() { f } / expected!("declaration")
        pub rule compilation_unit() -> &'a ASTNode<'a> = _ d:(d:decl() _ { d })* {
            let decls = arena.alloc_slice_copy(d.as_slice());
            arena.alloc(ASTNode::new((0, 0), NodeKind::Program(decls)))
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
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("1", &arena, &symbols);
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
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("1i32", &arena, &symbols);
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
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("1i64", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitInt(1, IntegerSuffix::I64),
                ..
            })
        ));
    }

    #[test]
    pub fn literal_float() {
        let arena = Bump::new();
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("1.0", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitFloat(1.0, FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitFloat(1., FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.0e10", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitFloat(1.0e10, FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.0f32", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitFloat(1.0, FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.0f64", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::LitFloat(1.0, FloatSuffix::F64),
                ..
            })
        ));
    }

    #[test]
    pub fn literal_string() {
        let arena = Bump::new();
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("\"hello\"", &arena, &symbols).unwrap();
        match ast {
            ASTNode {
                kind: NodeKind::LitString(s),
                ..
            } => {
                assert_eq!(symbols.resolve(*s), "hello");
            }
            _ => panic!(),
        };

        let ast = saga_parser::expr("\"hello\\n\"", &arena, &symbols).unwrap();
        match ast {
            ASTNode {
                kind: NodeKind::LitString(s),
                ..
            } => {
                assert_eq!(symbols.resolve(*s), "hello\n");
            }
            _ => panic!(),
        };

        let ast = saga_parser::expr("\"hello\\\"\"", &arena, &symbols).unwrap();
        match ast {
            ASTNode {
                kind: NodeKind::LitString(s),
                ..
            } => {
                assert_eq!(symbols.resolve(*s), "hello\"");
            }
            _ => panic!(),
        };

        let ast = saga_parser::expr("\"hello\u{25ff}\"", &arena, &symbols).unwrap();
        match ast {
            ASTNode {
                kind: NodeKind::LitString(s),
                ..
            } => {
                assert_eq!(symbols.resolve(*s), "hello◿");
            }
            _ => panic!(),
        };
    }

    #[test]
    pub fn binop_add() {
        let arena = Bump::new();
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("1.0 + 5", &arena, &symbols);
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
    pub fn binop_err() {
        let arena = Bump::new();
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::expr("1.0 + +", &arena, &symbols).unwrap_err();
        println!("{:?}", ast);
        assert_eq!(ast.location.offset, 6);
        println!("{:?}", ast.expected);
    }

    #[test]
    pub fn param_list() {
        let arena = Bump::new();
        let symbols = decl::InternedSymbols::new();
        let ast = saga_parser::param_list("()", &arena, &symbols).unwrap();
        assert_eq!(ast.len(), 0);

        let ast = saga_parser::param_list("(a: bool)", &arena, &symbols).unwrap();
        assert_eq!(ast.len(), 1);

        let ast = saga_parser::param_list("(a: bool,)", &arena, &symbols).unwrap();
        assert_eq!(ast.len(), 1);

        let ast = saga_parser::param_list("(a: bool, b: bool)", &arena, &symbols).unwrap();
        assert_eq!(ast.len(), 2);

        let ast = saga_parser::param_list("(a: bool, b: bool,)", &arena, &symbols).unwrap();
        assert_eq!(ast.len(), 2);
    }
}
