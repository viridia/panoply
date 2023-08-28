use std::fmt::{self};

use serde::{de::Visitor, Deserialize};
use winnow::{ascii::space1, combinator::separated1, PResult, Parser};

use super::expr::Expr;

/// Helper class for serializing lists of exprs
pub struct ExprList(pub Vec<Expr>);

impl ExprList {
    pub fn to_expr(&self) -> Expr {
        Expr::List(self.0.clone())
    }
}

fn parse_expr_list(input: &mut &str) -> PResult<ExprList> {
    separated1(Expr::parser, space1)
        .map(|items| ExprList(items))
        .parse_next(input)
}

impl std::str::FromStr for ExprList {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_expr_list
            .parse(input.trim())
            .map_err(|e| e.to_string())
    }
}

impl<'de> Deserialize<'de> for ExprList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ExprVisitor)
    }
}

struct ExprVisitor;

impl<'de> Visitor<'de> for ExprVisitor {
    type Value = ExprList;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("CSS expression list")
    }

    // TODO: Support JSON list of exprs

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s.parse::<ExprList>() {
            Ok(expr) => Ok(expr),
            Err(_) => Err(E::invalid_type(
                serde::de::Unexpected::Other("expr"),
                &"Expression list",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui;

    use super::*;

    #[test]
    fn test_parse_expr_list() {
        assert_eq!(
            "0px auto 1%".parse::<ExprList>().unwrap().to_expr(),
            Expr::List(vec![
                Expr::Length(ui::Val::Px(0.)),
                Expr::Ident("auto".to_string()),
                Expr::Length(ui::Val::Percent(1.))
            ])
        );
    }
}
