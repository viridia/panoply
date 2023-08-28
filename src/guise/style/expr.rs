use bevy::{prelude::Color, ui::UiRect};
use serde::Serialize;
use winnow::{
    ascii::space0,
    combinator::{alt, cut_err, opt, preceded},
    error::StrContext,
    token::{one_of, take_while},
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

    /// A color value
    Color(ColorValue),

    /// A reference to an asset
    AssetPath(String),

    /// Top, Right, Bottom, Left.
    Rect(UiRect),

    /// A reference to a named style variable.
    Var(String),
    // Other CSS properties
    // Angle
    // Time

    // FUNCTIONS
    // RGB
    // RGBA
    // HSL
    // HSLA
    // CALC
    // LIGHTEN
    // DARKEN
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

impl Expr {
    // Fold constants with type hint.
    // pub fn fold_constants(&self, fallback: &Self) -> Self {
    //     self.clone()
    // }

    pub fn as_length(&self) -> Self {
        match self {
            Expr::Length(_) => self.clone(),
            _ => Expr::Length(ui::Val::Auto),
        }
    }
}

fn parse_hex_color_digits<'s>(input: &mut &'s str) -> PResult<&'s str> {
    take_while(1..8, ('0'..='9', 'a'..='f', 'A'..='F')).parse_next(input)
}

fn parse_decimal_digits<'s>(input: &mut &'s str) -> PResult<&'s str> {
    take_while(1.., '0'..='9').parse_next(input)
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

fn parse_color_fn<'s>(input: &mut &'s str) -> PResult<&'s str> {
    alt(("rgba", "rgb", "hsla", "hsl")).parse_next(input)
}

fn parse_color_ctor<'s>(input: &mut &'s str) -> PResult<Expr> {
    (
        parse_color_fn,
        preceded((space0, '(', space0), parse_number),
        preceded((space0, opt((',', space0))), parse_number),
        preceded((space0, opt((',', space0))), parse_number),
        opt(preceded(
            (space0, opt(one_of((',', '/'))), space0),
            parse_number,
        )),
        (space0, ')', space0),
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

fn parse_color_ctor2<'s>(input: &mut &'s str) -> PResult<&'s str> {
    (
        parse_color_fn,
        '(', // preceded((space0, '(', space0), parse_number),
    )
        .recognize()
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

fn parse_expr(input: &mut &str) -> PResult<Expr> {
    alt((
        parse_hex_color,
        parse_length,
        parse_var_ref,
        parse_color_ctor,
        parse_ident,
    ))
    .parse_next(input)
}

impl std::str::FromStr for Expr {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_expr.parse(input).map_err(|e| e.to_string())
    }
}

impl ToString for Expr {
    fn to_string(&self) -> String {
        match self {
            Expr::Ident(name) => name.clone(),
            Expr::Number(n) => n.to_string(),
            Expr::Length(_) => todo!(),
            Expr::Color(_) => todo!(),
            Expr::AssetPath(_) => todo!(),
            Expr::Rect(_) => todo!(),
            Expr::Var(name) => format!("var(--{})", name),
        }
    }
}

impl Serialize for Expr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(parse_expr.parse_peek("1.e2"), Ok(("", Expr::Number(100.0))));
        assert_eq!(parse_expr.parse_peek("1.e-2"), Ok(("", Expr::Number(0.01))));
        assert_eq!(parse_expr.parse_peek("1e2"), Ok(("", Expr::Number(100.0))));
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
}
