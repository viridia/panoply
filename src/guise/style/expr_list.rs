use std::fmt::{self};

use bevy::ui::{self, UiRect};
use serde::{de::Visitor, ser::SerializeSeq, Deserialize, Serialize};
use winnow::{ascii::space1, combinator::separated1, PResult, Parser};

use crate::guise::coerce::Coerce;

use super::UntypedExpr;

/// A list of expressions
#[derive(Debug, Clone, PartialEq)]
pub struct ExprList(pub Box<[UntypedExpr]>);

impl ExprList {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn from_exprs(exprs: &[UntypedExpr]) -> Self {
        Self(Box::from(exprs))
    }
}

impl Coerce<UiRect> for ExprList {
    fn coerce(&self) -> Option<UiRect> {
        if self.len() > 0 {
            let top: ui::Val = self.0[0].coerce()?;
            let right: ui::Val = if self.len() > 1 {
                self.0[1].coerce()?
            } else {
                top
            };
            let bottom: ui::Val = if self.len() > 2 {
                self.0[2].coerce()?
            } else {
                top
            };
            let left: ui::Val = if self.len() > 3 {
                self.0[3].coerce()?
            } else {
                right
            };
            Some(UiRect {
                left,
                right,
                top,
                bottom,
            })
        } else {
            None
        }
    }
}

fn parse_expr_list(input: &mut &str) -> PResult<ExprList> {
    separated1(UntypedExpr::parser, space1)
        .map(|items: Vec<UntypedExpr>| ExprList(items.into_boxed_slice()))
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

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ExprList::from_exprs(&[UntypedExpr::Number(v as f32)]))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ExprList::from_exprs(&[UntypedExpr::Number(v as f32)]))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ExprList::from_exprs(&[UntypedExpr::Number(v as f32)]))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ExprList::from_exprs(&[UntypedExpr::Number(v as f32)]))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ExprList::from_exprs(&[UntypedExpr::Number(v)]))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ExprList::from_exprs(&[UntypedExpr::Number(v as f32)]))
    }

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

impl Serialize for ExprList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.len() == 1 {
            return self.0.serialize(serializer);
        }
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for item in self.0.iter() {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}
