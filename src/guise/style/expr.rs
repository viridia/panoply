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
    PositionType(ui::PositionType),
    OverflowAxis(ui::OverflowAxis),
    Direction(ui::Direction),
    AlignItems(ui::AlignItems),
    AlignContent(ui::AlignContent),
    AlignSelf(ui::AlignSelf),
    JustifyItems(ui::JustifyItems),
    JustifyContent(ui::JustifyContent),
    JustifySelf(ui::JustifySelf),
    FlexDirection(ui::FlexDirection),
    FlexWrap(ui::FlexWrap),
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
    Position,
    OverflowAxis,
    Direction,
    AlignItems,
    AlignContent,
    AlignSelf,
    JustifyItems,
    JustifyContent,
    JustifySelf,
    FlexDirection,
    FlexWrap,
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

    /// Evaluate the expression and coerce to a `ui::PositionType`
    pub fn into_position(&self) -> Option<ui::PositionType> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "relative" => Some(ui::PositionType::Relative),
                "absolute" => Some(ui::PositionType::Absolute),
                _ => None,
            },
            Expr::PositionType(d) => Some(*d),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a `ui::OverflowAxis`
    pub fn into_overflow(&self) -> Option<ui::OverflowAxis> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "clip" => Some(ui::OverflowAxis::Clip),
                "visible" => Some(ui::OverflowAxis::Visible),
                _ => None,
            },
            Expr::OverflowAxis(d) => Some(*d),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a `ui::Direction`
    pub fn into_direction(&self) -> Option<ui::Direction> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "inherit" => Some(ui::Direction::Inherit),
                "ltr" => Some(ui::Direction::LeftToRight),
                "rtl" => Some(ui::Direction::RightToLeft),
                _ => None,
            },
            Expr::Direction(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_align_items(&self) -> Option<ui::AlignItems> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "default" => Some(ui::AlignItems::Default),
                "start" => Some(ui::AlignItems::Start),
                "end" => Some(ui::AlignItems::End),
                "flex-start" => Some(ui::AlignItems::FlexStart),
                "flex-end" => Some(ui::AlignItems::FlexEnd),
                "center" => Some(ui::AlignItems::Center),
                "baseline" => Some(ui::AlignItems::Baseline),
                "stretch" => Some(ui::AlignItems::Stretch),
                _ => None,
            },
            Expr::AlignItems(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_align_content(&self) -> Option<ui::AlignContent> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "default" => Some(ui::AlignContent::Default),
                "start" => Some(ui::AlignContent::Start),
                "end" => Some(ui::AlignContent::End),
                "flex-start" => Some(ui::AlignContent::FlexStart),
                "flex-end" => Some(ui::AlignContent::FlexEnd),
                "center" => Some(ui::AlignContent::Center),
                "space-between" => Some(ui::AlignContent::SpaceBetween),
                "space-around" => Some(ui::AlignContent::SpaceAround),
                "space-evenly" => Some(ui::AlignContent::SpaceEvenly),
                "stretch" => Some(ui::AlignContent::Stretch),
                _ => None,
            },
            Expr::AlignContent(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_align_self(&self) -> Option<ui::AlignSelf> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "auto" => Some(ui::AlignSelf::Auto),
                "start" => Some(ui::AlignSelf::Start),
                "end" => Some(ui::AlignSelf::End),
                "flex-start" => Some(ui::AlignSelf::FlexStart),
                "flex-end" => Some(ui::AlignSelf::FlexEnd),
                "center" => Some(ui::AlignSelf::Center),
                "baseline" => Some(ui::AlignSelf::Baseline),
                "stretch" => Some(ui::AlignSelf::Stretch),
                _ => None,
            },
            Expr::AlignSelf(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_justify_items(&self) -> Option<ui::JustifyItems> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "default" => Some(ui::JustifyItems::Default),
                "start" => Some(ui::JustifyItems::Start),
                "end" => Some(ui::JustifyItems::End),
                "center" => Some(ui::JustifyItems::Center),
                "baseline" => Some(ui::JustifyItems::Baseline),
                "stretch" => Some(ui::JustifyItems::Stretch),
                _ => None,
            },
            Expr::JustifyItems(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_justify_content(&self) -> Option<ui::JustifyContent> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "default" => Some(ui::JustifyContent::Default),
                "start" => Some(ui::JustifyContent::Start),
                "end" => Some(ui::JustifyContent::End),
                "flex-start" => Some(ui::JustifyContent::FlexStart),
                "flex-end" => Some(ui::JustifyContent::FlexEnd),
                "center" => Some(ui::JustifyContent::Center),
                "space-between" => Some(ui::JustifyContent::SpaceBetween),
                "space-around" => Some(ui::JustifyContent::SpaceAround),
                "space-evenly" => Some(ui::JustifyContent::SpaceEvenly),
                _ => None,
            },
            Expr::JustifyContent(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_justify_self(&self) -> Option<ui::JustifySelf> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "auto" => Some(ui::JustifySelf::Auto),
                "start" => Some(ui::JustifySelf::Start),
                "end" => Some(ui::JustifySelf::End),
                "center" => Some(ui::JustifySelf::Center),
                "baseline" => Some(ui::JustifySelf::Baseline),
                "stretch" => Some(ui::JustifySelf::Stretch),
                _ => None,
            },
            Expr::JustifySelf(d) => Some(*d),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a `ui::FlexDirection`
    pub fn into_flex_direction(&self) -> Option<ui::FlexDirection> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "row" => Some(ui::FlexDirection::Row),
                "row-reverse" => Some(ui::FlexDirection::RowReverse),
                "column" => Some(ui::FlexDirection::Column),
                "column-reverse" => Some(ui::FlexDirection::ColumnReverse),
                _ => None,
            },
            Expr::FlexDirection(d) => Some(*d),
            _ => None,
        }
    }

    /// Evaluate the expression and coerce to a `ui::FlexWrap`
    pub fn into_flex_wrap(&self) -> Option<ui::FlexWrap> {
        match self {
            Expr::Ident(ref n) => match n.as_str() {
                "nowrap" => Some(ui::FlexWrap::NoWrap),
                "wrap" => Some(ui::FlexWrap::Wrap),
                "wrap-reverse" => Some(ui::FlexWrap::WrapReverse),
                _ => None,
            },
            Expr::FlexWrap(d) => Some(*d),
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
                let top = coerce::<ui::Val>(&v[0])?;
                let right = if v.len() > 1 {
                    coerce::<ui::Val>(&v[1])?
                } else {
                    top
                };
                let bottom = if v.len() > 2 {
                    coerce::<ui::Val>(&v[2])?
                } else {
                    top
                };
                let left = if v.len() > 3 {
                    coerce::<ui::Val>(&v[3])?
                } else {
                    right
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
                let opt = coerce::<ui::Display>(self);
                if let Some(disp) = opt {
                    *self = Self::Display(disp)
                }
            }
            TypeHint::Position => {
                let opt = self.into_position();
                if let Some(disp) = opt {
                    *self = Self::PositionType(disp)
                }
            }
            TypeHint::OverflowAxis => {
                let opt = self.into_overflow();
                if let Some(disp) = opt {
                    *self = Self::OverflowAxis(disp)
                }
            }
            TypeHint::Direction => {
                let opt = self.into_direction();
                if let Some(disp) = opt {
                    *self = Self::Direction(disp)
                }
            }
            TypeHint::AlignItems => {
                let opt = self.into_align_items();
                if let Some(disp) = opt {
                    *self = Self::AlignItems(disp)
                }
            }
            TypeHint::AlignContent => {
                let opt = self.into_align_content();
                if let Some(disp) = opt {
                    *self = Self::AlignContent(disp)
                }
            }
            TypeHint::AlignSelf => {
                let opt = coerce::<ui::AlignSelf>(self);
                if let Some(disp) = opt {
                    *self = Self::AlignSelf(disp)
                }
            }
            TypeHint::JustifyItems => {
                let opt = self.into_justify_items();
                if let Some(disp) = opt {
                    *self = Self::JustifyItems(disp)
                }
            }
            TypeHint::JustifyContent => {
                let opt = self.into_justify_content();
                if let Some(disp) = opt {
                    *self = Self::JustifyContent(disp)
                }
            }
            TypeHint::JustifySelf => {
                let opt = self.into_justify_self();
                if let Some(disp) = opt {
                    *self = Self::JustifySelf(disp)
                }
            }
            TypeHint::FlexDirection => {
                let opt = self.into_flex_direction();
                if let Some(v) = opt {
                    *self = Self::FlexDirection(v)
                }
            }
            TypeHint::FlexWrap => {
                let opt = self.into_flex_wrap();
                if let Some(v) = opt {
                    *self = Self::FlexWrap(v)
                }
            }
        }
        self
    }
}

pub struct Coerce;

pub trait CoerceImpl<T> {
    fn coerce(e: &Expr) -> Option<T>;
}

impl CoerceImpl<i32> for Coerce {
    fn coerce(e: &Expr) -> Option<i32> {
        match e {
            Expr::Number(v) => Some(*v as i32),
            _ => None,
        }
    }
}

impl CoerceImpl<f32> for Coerce {
    fn coerce(e: &Expr) -> Option<f32> {
        match e {
            Expr::Number(v) => Some(*v),
            _ => None,
        }
    }
}

impl CoerceImpl<ColorValue> for Coerce {
    fn coerce(e: &Expr) -> Option<ColorValue> {
        match e {
            Expr::Color(c) => Some(*c),
            _ => None,
        }
    }
}

impl CoerceImpl<ui::Val> for Coerce {
    fn coerce(e: &Expr) -> Option<ui::Val> {
        match e {
            Expr::Length(v) => Some(*v),
            Expr::Number(v) => Some(ui::Val::Px(*v)),
            Expr::Ident(v) if v == "auto" => Some(ui::Val::Auto),
            _ => None,
        }
    }
}

impl CoerceImpl<ui::Display> for Coerce {
    fn coerce(e: &Expr) -> Option<ui::Display> {
        match e {
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
}

impl CoerceImpl<ui::AlignSelf> for Coerce {
    fn coerce(e: &Expr) -> Option<ui::AlignSelf> {
        match e {
            Expr::Ident(ref n) => match n.as_str() {
                "auto" => Some(ui::AlignSelf::Auto),
                "start" => Some(ui::AlignSelf::Start),
                "end" => Some(ui::AlignSelf::End),
                "flex-start" => Some(ui::AlignSelf::FlexStart),
                "flex-end" => Some(ui::AlignSelf::FlexEnd),
                "center" => Some(ui::AlignSelf::Center),
                "baseline" => Some(ui::AlignSelf::Baseline),
                "stretch" => Some(ui::AlignSelf::Stretch),
                _ => None,
            },
            Expr::AlignSelf(d) => Some(*d),
            _ => None,
        }
    }
}

pub fn coerce<T>(e: &Expr) -> Option<T>
where
    Coerce: CoerceImpl<T>,
{
    Coerce::coerce(e)
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
            Expr::PositionType(d) => match d {
                ui::PositionType::Relative => write!(f, "relative"),
                ui::PositionType::Absolute => write!(f, "absolute"),
            },
            Expr::OverflowAxis(d) => match d {
                ui::OverflowAxis::Clip => write!(f, "clip"),
                ui::OverflowAxis::Visible => write!(f, "visible"),
            },
            Expr::Direction(d) => match d {
                ui::Direction::Inherit => write!(f, "inherit"),
                ui::Direction::LeftToRight => write!(f, "ltr"),
                ui::Direction::RightToLeft => write!(f, "rtl"),
            },
            Expr::AlignItems(d) => match d {
                ui::AlignItems::Default => write!(f, "default"),
                ui::AlignItems::Start => write!(f, "start"),
                ui::AlignItems::End => write!(f, "end"),
                ui::AlignItems::FlexStart => write!(f, "flex-start"),
                ui::AlignItems::FlexEnd => write!(f, "flex-end"),
                ui::AlignItems::Center => write!(f, "center"),
                ui::AlignItems::Baseline => write!(f, "baseline"),
                ui::AlignItems::Stretch => write!(f, "stretch"),
            },
            Expr::AlignContent(d) => match d {
                ui::AlignContent::Default => write!(f, "default"),
                ui::AlignContent::Start => write!(f, "start"),
                ui::AlignContent::End => write!(f, "end"),
                ui::AlignContent::FlexStart => write!(f, "flex-start"),
                ui::AlignContent::FlexEnd => write!(f, "flex-end"),
                ui::AlignContent::Center => write!(f, "center"),
                ui::AlignContent::SpaceBetween => write!(f, "space-between"),
                ui::AlignContent::SpaceAround => write!(f, "space-around"),
                ui::AlignContent::SpaceEvenly => write!(f, "space-evenly"),
                ui::AlignContent::Stretch => write!(f, "stretch"),
            },
            Expr::AlignSelf(d) => match d {
                ui::AlignSelf::Auto => write!(f, "auto"),
                ui::AlignSelf::Start => write!(f, "start"),
                ui::AlignSelf::End => write!(f, "end"),
                ui::AlignSelf::FlexStart => write!(f, "flex-start"),
                ui::AlignSelf::FlexEnd => write!(f, "flex-end"),
                ui::AlignSelf::Center => write!(f, "center"),
                ui::AlignSelf::Baseline => write!(f, "baseline"),
                ui::AlignSelf::Stretch => write!(f, "stretch"),
            },
            Expr::JustifyItems(d) => match d {
                ui::JustifyItems::Default => write!(f, "default"),
                ui::JustifyItems::Start => write!(f, "start"),
                ui::JustifyItems::End => write!(f, "end"),
                ui::JustifyItems::Center => write!(f, "center"),
                ui::JustifyItems::Baseline => write!(f, "baseline"),
                ui::JustifyItems::Stretch => write!(f, "stretch"),
            },
            Expr::JustifyContent(d) => match d {
                ui::JustifyContent::Default => write!(f, "default"),
                ui::JustifyContent::Start => write!(f, "start"),
                ui::JustifyContent::End => write!(f, "end"),
                ui::JustifyContent::FlexStart => write!(f, "flex-start"),
                ui::JustifyContent::FlexEnd => write!(f, "flex-end"),
                ui::JustifyContent::Center => write!(f, "center"),
                ui::JustifyContent::SpaceBetween => write!(f, "space-between"),
                ui::JustifyContent::SpaceAround => write!(f, "space-around"),
                ui::JustifyContent::SpaceEvenly => write!(f, "space-evenly"),
            },
            Expr::JustifySelf(d) => match d {
                ui::JustifySelf::Auto => write!(f, "auto"),
                ui::JustifySelf::Start => write!(f, "start"),
                ui::JustifySelf::End => write!(f, "end"),
                ui::JustifySelf::Center => write!(f, "center"),
                ui::JustifySelf::Baseline => write!(f, "baseline"),
                ui::JustifySelf::Stretch => write!(f, "stretch"),
            },
            Expr::FlexDirection(d) => match d {
                ui::FlexDirection::Row => write!(f, "row"),
                ui::FlexDirection::RowReverse => write!(f, "row-reverse"),
                ui::FlexDirection::Column => write!(f, "column"),
                ui::FlexDirection::ColumnReverse => write!(f, "column-reverse"),
            },
            Expr::FlexWrap(d) => match d {
                ui::FlexWrap::NoWrap => write!(f, "nowrap"),
                ui::FlexWrap::Wrap => write!(f, "wrap"),
                ui::FlexWrap::WrapReverse => write!(f, "wrap-reverse"),
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
