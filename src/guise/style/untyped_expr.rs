use std::fmt::{self, Debug};

use bevy::{asset::AssetPath, prelude::*};
use serde::{de::Visitor, Deserialize, Serialize};
use winnow::{
    ascii::space0,
    combinator::{alt, cut_err, delimited, opt, preceded},
    error::StrContext,
    token::{one_of, take_until1, take_while},
    PResult, Parser,
};

use bevy::ui;

use super::{asset_ref::AssetRef, coerce::Coerce, color::ColorValue};

/// An expression which represents the possible values of a style attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum UntypedExpr {
    /// An identifier
    Ident(String),

    /// A floating-point number
    Number(f32),

    /// A length
    Length(ui::Val),

    /// A list of expressions
    // List(Box<[UntypedExpr]>),

    /// A color value
    Color(ColorValue),

    /// A reference to an asset: "asset(path)"
    Asset(AssetRef),

    /// A reference to a named style variable.
    Var(String),
}

impl UntypedExpr {
    pub fn parser(input: &mut &str) -> PResult<UntypedExpr> {
        alt((
            parse_hex_color,
            parse_length,
            parse_var_ref,
            parse_color_ctor,
            parse_asset,
            parse_ident,
        ))
        .parse_next(input)
    }

    /// Resolve relative paths to full paths
    pub fn resolve_asset_paths(&mut self, base: &AssetPath) {
        match self {
            // UntypedExpr::List(exprs) => exprs
            //     .iter_mut()
            //     .for_each(|expr| expr.resolve_asset_paths(base)),
            UntypedExpr::Asset(ref mut asset_ref) => asset_ref.resolve_asset_path(base),
            _ => {}
        }
    }
}

impl Coerce<ui::Val> for UntypedExpr {
    fn coerce(&self) -> Option<ui::Val> {
        match self {
            Self::Length(v) => Some(*v),
            Self::Number(v) => Some(ui::Val::Px(*v)),
            Self::Ident(v) if v == "auto" => Some(ui::Val::Auto),
            _ => None,
        }
    }
}

fn parse_hex_color_digits<'s>(input: &mut &'s str) -> PResult<&'s str> {
    take_while(1..8, ('0'..='9', 'a'..='f', 'A'..='F')).parse_next(input)
}

fn parse_decimal_digits<'s>(input: &mut &'s str) -> PResult<&'s str> {
    take_while(1.., '0'..='9').parse_next(input)
}

fn parse_asset_key<'s>(input: &mut &'s str) -> PResult<&'s str> {
    take_until1(")").parse_next(input)
}

fn parse_exponent<'s>(input: &mut &'s str) -> PResult<()> {
    (
        one_of(['E', 'e']),
        opt(one_of(['+', '-'])),
        take_while(1.., '0'..='9'),
    )
        .void()
        .parse_next(input)
}

fn parse_number<'s>(input: &mut &'s str) -> PResult<f32> {
    alt((
        (opt('-'), '.', parse_decimal_digits, opt(parse_exponent)).recognize(),
        (
            opt('-'),
            parse_decimal_digits,
            opt(('.', opt(parse_decimal_digits))),
            opt(parse_exponent),
        )
            .recognize(),
    ))
    .map(|s| s.parse::<f32>().unwrap())
    .parse_next(input)
}

fn parse_hex_color(input: &mut &str) -> PResult<UntypedExpr> {
    (
        '#',
        cut_err(parse_hex_color_digits).context(StrContext::Label("color")),
    )
        .map(|(_, str)| match str.len() {
            3 | 4 | 6 | 8 => UntypedExpr::Color(ColorValue::Color(Color::hex(str).unwrap())),
            // TODO: Return error here? Not sure how to do that.
            _ => UntypedExpr::Color(ColorValue::Color(Color::NONE)),
        })
        .parse_next(input)
}

fn parse_color_ctor<'s>(input: &mut &'s str) -> PResult<UntypedExpr> {
    (
        alt(("rgba", "rgb", "hsla", "hsl")),
        preceded((space0, '(', space0), cut_err(parse_number)),
        preceded((space0, opt((',', space0))), parse_number),
        preceded((space0, opt((',', space0))), parse_number),
        opt(preceded(
            (space0, opt(one_of((',', '/'))), space0),
            parse_number,
        )),
        (space0, ')'),
    )
        .map(|(f, a1, a2, a3, a4, _)| match f {
            "rgba" | "rgb" => UntypedExpr::Color(ColorValue::Color(Color::Rgba {
                red: a1 / 255.0,
                green: a2 / 255.0,
                blue: a3 / 255.0,
                alpha: a4.unwrap_or(1.),
            })),
            "hsla" | "hsl" => UntypedExpr::Color(ColorValue::Color(Color::Hsla {
                hue: a1 / 360.0,
                saturation: a2 / 100.0,
                lightness: a3 / 100.0,
                alpha: a4.unwrap_or(1.),
            })),
            _ => unreachable!(),
        })
        .parse_next(input)
}

fn parse_asset<'s>(input: &mut &'s str) -> PResult<UntypedExpr> {
    ("asset", space0, delimited('(', parse_asset_key, ')'))
        .map(|(_, _, path)| UntypedExpr::Asset(AssetRef::new(path)))
        .parse_next(input)
}

fn parse_length<'s>(input: &mut &'s str) -> PResult<UntypedExpr> {
    (
        parse_number,
        opt(alt(("px", "%", "vh", "vw", "vmin", "vmax"))),
    )
        .map(|(f, suffix)| {
            if suffix.is_none() {
                UntypedExpr::Number(f)
            } else {
                match suffix {
                    Some("px") => UntypedExpr::Length(ui::Val::Px(f)),
                    Some("%") => UntypedExpr::Length(ui::Val::Percent(f)),
                    Some("vw") => UntypedExpr::Length(ui::Val::Vw(f)),
                    Some("vh") => UntypedExpr::Length(ui::Val::Vh(f)),
                    Some("vmin") => UntypedExpr::Length(ui::Val::VMin(f)),
                    Some("vmax") => UntypedExpr::Length(ui::Val::VMax(f)),
                    _ => unreachable!(),
                }
            }
        })
        .parse_next(input)
}

fn parse_name<'s>(input: &mut &'s str) -> PResult<&'s str> {
    (
        one_of(('A'..'Z', 'a'..'z', '_')),
        take_while(0.., ('A'..'Z', 'a'..'z', '0'..'9', '_', '-')),
    )
        .recognize()
        .parse_next(input)
}

fn parse_ident<'s>(input: &mut &'s str) -> PResult<UntypedExpr> {
    parse_name
        .map(|s| UntypedExpr::Ident(s.to_string()))
        .parse_next(input)
}

fn parse_var_ref<'s>(input: &mut &'s str) -> PResult<UntypedExpr> {
    ("var(--", parse_name, ")")
        .map(|(_, name, _)| UntypedExpr::Var(name.to_string()))
        .parse_next(input)
}

impl std::str::FromStr for UntypedExpr {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        UntypedExpr::parser
            .parse(input.trim())
            .map_err(|e| e.to_string())
    }
}

impl fmt::Display for UntypedExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UntypedExpr::Ident(name) => write!(f, "{}", name),
            UntypedExpr::Number(n) => write!(f, "{}", n),
            UntypedExpr::Length(l) => match l {
                ui::Val::Auto => write!(f, "auto"),
                ui::Val::Px(v) => write!(f, "{}px", v),
                ui::Val::Percent(v) => write!(f, "{}%", v),
                ui::Val::Vw(v) => write!(f, "{}vw", v),
                ui::Val::Vh(v) => write!(f, "{}vh", v),
                ui::Val::VMin(v) => write!(f, "{}vmin", v),
                ui::Val::VMax(v) => write!(f, "{}vmax", v),
            },
            // UntypedExpr::List(r) => {
            //     for (i, x) in r.iter().enumerate() {
            //         if i != 0 {
            //             write!(f, " ")?
            //         }
            //         fmt::Display::fmt(&x, f)?
            //     }
            //     Ok(())
            // }
            UntypedExpr::Color(c) => fmt::Display::fmt(&c, f),
            UntypedExpr::Asset(_) => todo!(),
            UntypedExpr::Var(name) => write!(f, "var(--{})", name),
        }
    }
}

impl Serialize for UntypedExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Number(n) => {
                if n.round() == *n {
                    serializer.serialize_i32(*n as i32)
                } else {
                    serializer.serialize_f32(*n)
                }
            }
            Self::Length(ui::Val::Px(n)) => {
                if n.round() == *n {
                    serializer.serialize_i32(*n as i32)
                } else {
                    serializer.serialize_f32(*n)
                }
            }
            _ => serializer.collect_str(&self),
        }
    }
}

impl<'de> Deserialize<'de> for UntypedExpr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ExprVisitor)
    }
}

struct ExprVisitor;

impl<'de> Visitor<'de> for ExprVisitor {
    type Value = UntypedExpr;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("CSS expression")
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UntypedExpr::Number(v as f32))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UntypedExpr::Number(v as f32))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UntypedExpr::Number(v as f32))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UntypedExpr::Number(v as f32))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UntypedExpr::Number(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(UntypedExpr::Number(v as f32))
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s.parse::<UntypedExpr>() {
            Ok(expr) => Ok(expr),
            Err(_) => Err(E::invalid_type(
                serde::de::Unexpected::Str(s),
                &"CSS expression",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trailing_space() {
        assert_eq!(
            "#f00 ".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::RED))
        );
        assert_eq!(
            "1 ".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Number(1.)
        );
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(
            "#f00".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::RED))
        );
        assert_eq!(
            "#00f".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::BLUE))
        );
        // Invalid color value parsed as NONE
        assert_eq!(
            "#0f".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::NONE))
        );
    }

    #[test]
    fn test_parse_color_fn() {
        assert_eq!(
            "rgba( 255 255 255 )".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 1.)))
        );
        assert_eq!(
            "rgba(255, 255, 255)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 1.)))
        );
        assert_eq!(
            "rgba(255, 255, 255, 0.5)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            "rgba(255 255 255 / 0.5)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            "rgb(255 255 255 / 0.5)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            "hsla(360 100 100 / 0.5)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Color(ColorValue::Color(Color::hsla(1., 1., 1., 0.5)))
        );
    }

    #[test]
    fn test_parse_int() {
        assert_eq!("1".parse::<UntypedExpr>().unwrap(), UntypedExpr::Number(1.));
        assert_eq!(
            "77".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Number(77.)
        );
    }

    #[test]
    fn test_parse_float() {
        assert_eq!(
            "1.0".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Number(1.0)
        );
        assert_eq!(
            ".1".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Number(0.1)
        );
        assert_eq!(
            "1.".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Number(1.0)
        );
        assert_eq!(
            UntypedExpr::parser.parse_peek("1.e2"),
            Ok(("", UntypedExpr::Number(100.0)))
        );
        assert_eq!(
            UntypedExpr::parser.parse_peek("1.e-2"),
            Ok(("", UntypedExpr::Number(0.01)))
        );
        assert_eq!(
            UntypedExpr::parser.parse_peek("1e2"),
            Ok(("", UntypedExpr::Number(100.0)))
        );
        assert_eq!(
            "-1.".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Number(-1.0)
        );
    }

    #[test]
    fn test_parse_length() {
        assert_eq!(
            "1px".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Length(ui::Val::Px(1.))
        );
        assert_eq!(
            "10%".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Length(ui::Val::Percent(10.))
        );
        assert_eq!(
            "7vw".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Length(ui::Val::Vw(7.))
        );
        assert_eq!(
            "7e-1vh".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Length(ui::Val::Vh(0.7))
        );
        assert_eq!(
            "7vmin".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Length(ui::Val::VMin(7.))
        );
        assert_eq!(
            "7vmax".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Length(ui::Val::VMax(7.))
        );
    }

    #[test]
    fn test_parse_ident() {
        assert_eq!(
            "foo".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Ident("foo".to_string())
        );
    }

    #[test]
    fn test_parse_var_ref() {
        assert_eq!(
            "var(--foo)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Var("foo".to_string())
        );
    }

    #[test]
    fn test_parse_asset() {
        assert_eq!(
            "asset(../image.png)".parse::<UntypedExpr>().unwrap(),
            UntypedExpr::Asset(AssetRef::new("../image.png"))
        );
    }
}
