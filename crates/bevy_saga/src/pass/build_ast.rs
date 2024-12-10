use bumpalo::Bump;
use lazy_static::lazy_static;
use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc, Op, PrattParser},
};

use crate::{
    ast::{self, ASTNode, FloatSuffix, IntegerSuffix},
    location::TokenLocation,
    oper::BinaryOp,
    parser::Rule,
};

lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use Assoc::*;
        use Rule::*;

        PrattParser::new()
            .op(Op::infix(logical_and, Left))
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
            .op(Op::infix(multiply, Left) | Op::infix(divide, Left) | Op::infix(modulus, Left))
            .op(Op::prefix(neg))
            .op(Op::prefix(not))
            .op(Op::prefix(complement))
            // .op(Op::infix(power, Right))
    };
}

pub(crate) fn build_ast<'a>(arena: &'a Bump, expression: Pairs<Rule>) -> &'a ASTNode<'a> {
    PRATT_PARSER
        .map_primary(|pair| {
            let location: TokenLocation = pair.as_span().into();
            match pair.as_rule() {
                Rule::unit => {
                    println!("unit: {:?}", pair);
                    let mut decls: Vec<&'a ASTNode<'a>> = Vec::new();
                    pair.into_inner().for_each(|decl| match decl.as_rule() {
                        Rule::decl => {
                            println!("decl: {:?}", decl.as_str());
                            decls.push(build_ast(arena, decl.into_inner()));
                        }
                        _ => todo!("to_ast: {:?}", decl.as_rule()),
                    });
                    let decls = arena.alloc_slice_copy(decls.as_slice());
                    arena.alloc(ASTNode::new(location, ast::NodeKind::Unit(decls)))
                }
                Rule::expr => build_ast(arena, pair.into_inner()),
                Rule::binary => build_ast(arena, pair.into_inner()),
                Rule::unary => build_ast(arena, pair.into_inner()),
                Rule::primary => build_ast(arena, pair.into_inner()),
                Rule::float => {
                    let value = arena.alloc_str(pair.as_str());
                    arena.alloc(ASTNode::new(
                        location,
                        ast::NodeKind::ConstFloat(value, FloatSuffix::F32),
                    ))
                }
                Rule::integer => {
                    let value = arena.alloc_str(pair.as_str());
                    arena.alloc(ASTNode::new(
                        location,
                        ast::NodeKind::ConstInteger(value, IntegerSuffix::Unsized),
                    ))
                }
                _ => todo!("to_ast: {:?}", pair.as_rule()),
            }
        })
        .map_infix(|lhs, op, rhs| {
            let location = lhs.location.union(&rhs.location);
            arena.alloc(ASTNode::new(
                location,
                ast::NodeKind::BinaryExpr {
                    op: match op.as_rule() {
                        Rule::add => BinaryOp::Add,
                        Rule::subtract => BinaryOp::Sub,
                        Rule::multiply => BinaryOp::Mul,
                        Rule::divide => BinaryOp::Div,
                        Rule::modulus => BinaryOp::Mod,
                        _ => unreachable!(),
                    },
                    lhs,
                    rhs,
                },
            ))
        })
        .parse(expression)
}
