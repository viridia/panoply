use std::fmt::{self, Debug};

use bevy::{prelude::Color, ui::UiRect};
use serde::{de::Visitor, Deserialize, Serialize};
use winnow::{
    ascii::space0,
    combinator::{alt, cut_err, delimited, opt, preceded},
    error::StrContext,
    token::{one_of, take_until1, take_while},
    PResult, Parser,
};

use bevy::ui;

use super::color::ColorValue;

/// An expression which represents the possible values of a style attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// An identifier
    Ident(String),

    /// A floating-point number
    Number(f32),

    /// A length
    Length(ui::Val),

    /// A list of expressions
    List(Vec<Expr>),

    /// A color value
    Color(ColorValue),

    /// A reference to an asset: "asset(path)"
    Asset(String),

    /// A reference to a named style variable.
    Var(String),
    // Other CSS properties
    // Angle
    // Time

    // FUNCTIONS
    // CALC
    // LIGHTEN
    // DARKEN
    Display(ui::Display),
}

pub enum CssFn {
    Rgb,
    Rgba,
    Hsl,
    Hsla,
    Lighten,
    Darken,
    Calc,
    Max,
    Min,
    // Gradients
}

/// Type hints for optimization.
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum TypeHint {
    Length,
    Display,
}

impl Expr {
    pub fn parser(input: &mut &str) -> PResult<Expr> {
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

    /// Evaluate the expression and coerce to an int.
    pub fn into_i32(&self) -> Option<i32> {
        match self {
            Expr::Number(v) => Some(*v as i32),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a float.
    pub fn into_f32(&self) -> Option<f32> {
        match self {
            Expr::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a length.
    pub fn into_length(&self) -> Option<ui::Val> {
        match self {
            Expr::Length(v) => Some(*v),
            Expr::Number(v) => Some(ui::Val::Px(*v)),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a color
    pub fn into_color(&self) -> Option<ColorValue> {
        match self {
            Expr::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a ui::Display
    pub fn into_display(&self) -> Option<ui::Display> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "grid" => Some(ui::Display::Grid),
                "flex" => Some(ui::Display::Flex),
                "none" => Some(ui::Display::None),
                _ => None,
            },
            Expr::Display(d) => Some(*d),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a length.
    pub fn into_uirect(&self) -> Option<ui::UiRect> {
        match self {
            Expr::Length(v) => Some(UiRect {
                left: *v,
                right: *v,
                top: *v,
                bottom: *v,
            }),
            Expr::Number(v) => Some(UiRect {
                left: ui::Val::Px(*v),
                right: ui::Val::Px(*v),
                top: ui::Val::Px(*v),
                bottom: ui::Val::Px(*v),
            }),
            Expr::List(v) if v.len() > 0 => {
                let top = v[0].into_length()?;
                let right = if v.len() > 1 {
                    top
                } else {
                    v[1].into_length()?
                };
                let bottom = if v.len() > 2 {
                    top
                } else {
                    v[2].into_length()?
                };
                let left = if v.len() > 3 {
                    right
                } else {
                    v[3].into_length()?
                };
                Some(UiRect {
                    left,
                    right,
                    top,
                    bottom,
                })
            }
            _ => None,
        }
    }

    /// Fold constants
    pub fn into_display_const(self) -> Self {
        self.into_display().map_or(self, |f| Self::Display(f))
    }

    /// Optimize constant expression with type hints
    pub fn optimize(&mut self, hint: TypeHint) -> &Self {
        if let Self::List(l) = self {
            for x in l.iter_mut() {
                x.optimize(hint);
            }
            return self;
        }

        match hint {
            TypeHint::Length => {
                if let Self::Ident(s) = self {
                    if s == "auto" {
                        *self = Self::Length(ui::Val::Auto)
                    }
                }
            }
            TypeHint::Display => {
                let opt = self.into_display();
                if let Some(disp) = opt {
                    *self = Self::Display(disp)
                }
            }
        }
        self
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Self::Number(value as f32)
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

fn parse_hex_color(input: &mut &str) -> PResult<Expr> {
    (
        '#',
        cut_err(parse_hex_color_digits).context(StrContext::Label("color")),
    )
        .map(|(_, str)| match str.len() {
            3 | 4 | 6 | 8 => Expr::Color(ColorValue::Color(Color::hex(str).unwrap())),
            // TODO: Return error here? Not sure how to do that.
            _ => Expr::Color(ColorValue::Color(Color::NONE)),
        })
        .parse_next(input)
}

fn parse_color_ctor<'s>(input: &mut &'s str) -> PResult<Expr> {
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
            "rgba" | "rgb" => Expr::Color(ColorValue::Color(Color::Rgba {
                red: a1 / 255.0,
                green: a2 / 255.0,
                blue: a3 / 255.0,
                alpha: a4.unwrap_or(1.),
            })),
            "hsla" | "hsl" => Expr::Color(ColorValue::Color(Color::Hsla {
                hue: a1 / 360.0,
                saturation: a2 / 100.0,
                lightness: a3 / 100.0,
                alpha: a4.unwrap_or(1.),
            })),
            _ => unreachable!(),
        })
        .parse_next(input)
}

fn parse_asset<'s>(input: &mut &'s str) -> PResult<Expr> {
    ("asset", space0, delimited('(', parse_asset_key, ')'))
        .map(|(_, _, path)| Expr::Asset(path.to_string()))
        .parse_next(input)
}

fn parse_length<'s>(input: &mut &'s str) -> PResult<Expr> {
    (
        parse_number,
        opt(alt(("px", "%", "vh", "vw", "vmin", "vmax"))),
    )
        .map(|(f, suffix)| {
            if suffix.is_none() {
                Expr::Number(f)
            } else {
                match suffix {
                    Some("px") => Expr::Length(ui::Val::Px(f)),
                    Some("%") => Expr::Length(ui::Val::Percent(f)),
                    Some("vw") => Expr::Length(ui::Val::Vw(f)),
                    Some("vh") => Expr::Length(ui::Val::Vh(f)),
                    Some("vmin") => Expr::Length(ui::Val::VMin(f)),
                    Some("vmax") => Expr::Length(ui::Val::VMax(f)),
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

fn parse_ident<'s>(input: &mut &'s str) -> PResult<Expr> {
    parse_name
        .map(|s| Expr::Ident(s.to_string()))
        .parse_next(input)
}

fn parse_var_ref<'s>(input: &mut &'s str) -> PResult<Expr> {
    ("var(--", parse_name, ")")
        .map(|(_, name, _)| Expr::Var(name.to_string()))
        .parse_next(input)
}

impl std::str::FromStr for Expr {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Expr::parser.parse(input.trim()).map_err(|e| e.to_string())
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Ident(name) => write!(f, "{}", name),
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Length(l) => match l {
                ui::Val::Auto => write!(f, "auto"),
                ui::Val::Px(v) => write!(f, "{}px", v),
                ui::Val::Percent(v) => write!(f, "{}%", v),
                ui::Val::Vw(v) => write!(f, "{}vw", v),
                ui::Val::Vh(v) => write!(f, "{}vh", v),
                ui::Val::VMin(v) => write!(f, "{}vmin", v),
                ui::Val::VMax(v) => write!(f, "{}vmax", v),
            },
            Expr::List(r) => {
                for (i, x) in r.iter().enumerate() {
                    if i != 0 {
                        write!(f, " ")?
                    }
                    fmt::Display::fmt(&x, f)?
                }
                Ok(())
            }
            Expr::Color(c) => fmt::Display::fmt(&c, f),
            Expr::Asset(_) => todo!(),
            Expr::Display(d) => match d {
                ui::Display::Flex => write!(f, "flex"),
                ui::Display::Grid => write!(f, "grid"),
                ui::Display::None => write!(f, "none"),
            },
            Expr::Var(name) => write!(f, "var(--{})", name),
        }
    }
}

impl Serialize for Expr {
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

impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ExprVisitor)
    }
}

struct ExprVisitor;

impl<'de> Visitor<'de> for ExprVisitor {
    type Value = Expr;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("CSS expression")
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Expr::Number(v as f32))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Expr::Number(v as f32))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Expr::Number(v as f32))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Expr::Number(v as f32))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Expr::Number(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Expr::Number(v as f32))
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s.parse::<Expr>() {
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
            "#f00 ".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::RED))
        );
        assert_eq!("1 ".parse::<Expr>().unwrap(), Expr::Number(1.));
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(
            "#f00".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::RED))
        );
        assert_eq!(
            "#00f".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::BLUE))
        );
        // Invalid color value parsed as NONE
        assert_eq!(
            "#0f".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::NONE))
        );
    }

    #[test]
    fn test_parse_color_fn() {
        assert_eq!(
            "rgba( 255 255 255 )".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 1.)))
        );
        assert_eq!(
            "rgba(255, 255, 255)".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 1.)))
        );
        assert_eq!(
            "rgba(255, 255, 255, 0.5)".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            "rgba(255 255 255 / 0.5)".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            "rgb(255 255 255 / 0.5)".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            "hsla(360 100 100 / 0.5)".parse::<Expr>().unwrap(),
            Expr::Color(ColorValue::Color(Color::hsla(1., 1., 1., 0.5)))
        );
    }

    #[test]
    fn test_parse_int() {
        assert_eq!("1".parse::<Expr>().unwrap(), Expr::Number(1.));
        assert_eq!("77".parse::<Expr>().unwrap(), Expr::Number(77.));
    }

    #[test]
    fn test_parse_float() {
        assert_eq!("1.0".parse::<Expr>().unwrap(), Expr::Number(1.0));
        assert_eq!(".1".parse::<Expr>().unwrap(), Expr::Number(0.1));
        assert_eq!("1.".parse::<Expr>().unwrap(), Expr::Number(1.0));
        assert_eq!(
            Expr::parser.parse_peek("1.e2"),
            Ok(("", Expr::Number(100.0)))
        );
        assert_eq!(
            Expr::parser.parse_peek("1.e-2"),
            Ok(("", Expr::Number(0.01)))
        );
        assert_eq!(
            Expr::parser.parse_peek("1e2"),
            Ok(("", Expr::Number(100.0)))
        );
        assert_eq!("-1.".parse::<Expr>().unwrap(), Expr::Number(-1.0));
    }

    #[test]
    fn test_parse_length() {
        assert_eq!(
            "1px".parse::<Expr>().unwrap(),
            Expr::Length(ui::Val::Px(1.))
        );
        assert_eq!(
            "10%".parse::<Expr>().unwrap(),
            Expr::Length(ui::Val::Percent(10.))
        );
        assert_eq!(
            "7vw".parse::<Expr>().unwrap(),
            Expr::Length(ui::Val::Vw(7.))
        );
        assert_eq!(
            "7e-1vh".parse::<Expr>().unwrap(),
            Expr::Length(ui::Val::Vh(0.7))
        );
        assert_eq!(
            "7vmin".parse::<Expr>().unwrap(),
            Expr::Length(ui::Val::VMin(7.))
        );
        assert_eq!(
            "7vmax".parse::<Expr>().unwrap(),
            Expr::Length(ui::Val::VMax(7.))
        );
    }

    #[test]
    fn test_parse_ident() {
        assert_eq!(
            "foo".parse::<Expr>().unwrap(),
            Expr::Ident("foo".to_string())
        );
    }

    #[test]
    fn test_parse_var_ref() {
        assert_eq!(
            "var(--foo)".parse::<Expr>().unwrap(),
            Expr::Var("foo".to_string())
        );
    }

    #[test]
    fn test_parse_asset() {
        assert_eq!(
            "asset(../image.png)".parse::<Expr>().unwrap(),
            Expr::Asset("../image.png".to_string())
        );
    }
}
