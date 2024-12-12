use crate::ast::{
    ASTNode, DeclKind, FloatSuffix, FunctionParam, IntegerSuffix, NodeKind, TypeKind,
};
use crate::decl;
use crate::oper::BinaryOp;
use bumpalo::Bump;

peg::parser! {
    pub grammar saga_parser<'a, 's>(arena: &'a Bump, symbols: &'s decl::SymbolTable) for str {
        rule _() = quiet!{[' ' | '\n' | '\t']*}

        rule dec_literal() = n:$(['0'..='9']['0'..='9' | '_']*) { }
        rule name() -> decl::Symbol =
            n:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*)
            {
                symbols.intern(n)
            }

        rule lit_float() -> &'a ASTNode<'a> =
            start:position!()
            n:$(
                dec_literal() &(['.' | 'e' | 'f'])
                ("." ['0'..='9' | '_']*)?
                ("e" ['+' | '-']? dec_literal())?
            )
            s:("f32" { FloatSuffix::F32 } / "f64" { FloatSuffix::F64 })?
            end:position!()
        {
            let value = arena.alloc_str(n);
            arena.alloc(ASTNode::new((start, end), NodeKind::ConstFloat(value, s.unwrap_or(FloatSuffix::F32))))

        }
        rule lit_int() -> &'a ASTNode<'a> =
            start:position!()
            n:$(dec_literal())
            s:("i32" { IntegerSuffix::I32 } / "i64" { IntegerSuffix::I64 })?
            end:position!()
        {
            // let location = TokenLocation::new(start, end);
            let value = arena.alloc_str(n);
            arena.alloc(ASTNode::new((start, end), NodeKind::ConstInteger(value, s.unwrap_or(IntegerSuffix::Unsized))))
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
            / ident()
            / "(" _ e:expr() _ ")" { e }
            / expected!("expression")
        ) { e }

        rule binop() -> &'a ASTNode<'a> = precedence!{
            lhs:(@) _ "||" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::LogOr,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "&&" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::LogAnd,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "==" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Eq,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "!=" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Ne,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ ">" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Gt,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ ">=" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Ge,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "<" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Lt,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "<=" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Le,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "|" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::BitOr,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "^" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::BitXor,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "&" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::BitAnd,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "<<" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Shl,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ ">>" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Shr,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "+" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Add,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "-" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Sub,
                    lhs,
                    rhs,
                }))
            }
            --
            lhs:(@) _ "*" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Mul,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "/" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Div,
                    lhs,
                    rhs,
                }))
            }
            lhs:(@) _ "%" _ rhs:@ {
                let location = lhs.location.union(&rhs.location);
                arena.alloc(ASTNode::new(location, NodeKind::BinaryExpr {
                    op: BinaryOp::Mod,
                    lhs,
                    rhs,
                }))
            }
            --
            p:primary() { p }
        }

        pub rule expr() -> &'a ASTNode<'a> = l:(binop()) { l } / expected!("expression")
        rule empty_stmt() -> &'a ASTNode<'a> = ";" _ {
            arena.alloc(ASTNode::new((0, 0), NodeKind::Empty))
        }

        rule builtin_type() -> &'a ASTNode<'a> =
            start:position!()
            t:(   "bool" { TypeKind::Boolean }
                / "i32" { TypeKind::I32 }
                / "i64" { TypeKind::I64 }
                / "f32" { TypeKind::F32 }
                / "f64" { TypeKind::F64 }
            )
            end:position!()
            {
                arena.alloc(ASTNode::new((start, end), NodeKind::Type(t)))
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

        pub rule type_expr() -> &'a ASTNode<'a> = builtin_type() / type_name() / array_type()

        rule stmt() -> &'a ASTNode<'a> =
            start:position!()
            s:(
                s0: empty_stmt() { s0 }
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
            "fn" _ id:name() _ p:param_list() _ r:func_return()? _ b:func_body()
            end:position!()
        {
            let location = (start, end);
            arena.alloc(ASTNode::new(location, NodeKind::Decl(arena.alloc(DeclKind::Function {
                name: id,
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
    pub fn test_literal_int() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
        let ast = saga_parser::expr("1", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstInteger("1", IntegerSuffix::Unsized),
                ..
            })
        ));
        // assert_eq!(ast, Ok(vec![1, 1, 2, 3, 5, 8]));
    }

    #[test]
    pub fn test_literal_int_i32() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
        let ast = saga_parser::expr("1i32", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstInteger("1", IntegerSuffix::I32),
                ..
            })
        ));
    }

    #[test]
    pub fn test_literal_int_i64() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
        let ast = saga_parser::expr("1i64", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstInteger("1", IntegerSuffix::I64),
                ..
            })
        ));
    }

    #[test]
    pub fn test_literal_float() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
        let ast = saga_parser::expr("1.0", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstFloat("1.0", FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstFloat("1.", FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.0e10", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstFloat("1.0e10", FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.0f32", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstFloat("1.0", FloatSuffix::F32),
                ..
            })
        ));

        let ast = saga_parser::expr("1.0f64", &arena, &symbols);
        assert!(matches!(
            ast,
            Ok(ASTNode {
                kind: NodeKind::ConstFloat("1.0", FloatSuffix::F64),
                ..
            })
        ));
    }

    #[test]
    pub fn test_binop() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
        let ast = saga_parser::expr("1.0 + 5", &arena, &symbols);
        match ast {
            Ok(ASTNode {
                kind: NodeKind::BinaryExpr { op, lhs, rhs },
                ..
            }) => {
                assert_eq!(*op, BinaryOp::Add);
                assert!(matches!(
                    lhs.kind,
                    NodeKind::ConstFloat("1.0", FloatSuffix::F32)
                ));
                assert!(matches!(
                    rhs.kind,
                    NodeKind::ConstInteger("5", IntegerSuffix::Unsized)
                ));
            }
            _ => panic!(),
        }
    }

    #[test]
    pub fn test_binop_err() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
        let ast = saga_parser::expr("1.0 + +", &arena, &symbols).unwrap_err();
        println!("{:?}", ast);
        assert_eq!(ast.location.offset, 6);
        // assert!(ast.expected.tokens().in(&"expression"));
        println!("{:?}", ast.expected);
    }

    #[test]
    pub fn test_params() {
        let arena = Bump::new();
        let symbols = decl::SymbolTable::new();
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
