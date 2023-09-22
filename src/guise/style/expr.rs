use std::fmt::{self, Debug};

use serde::{Deserialize, Serialize};

use bevy::ui;

use super::{asset_ref::AssetRef, coerce::Coerce, color::ColorValue, untyped_expr::UntypedExpr};

/// An expression which represents the possible values of a style attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleExpr<T> {
    /// A constant value of the type of the expression.
    Constant(T),

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
    // Other CSS properties
    // Angle
    // Time

    // FUNCTIONS
    // CALC
    // LIGHTEN
    // DARKEN
}

impl<T> StyleExpr<T> {
    fn fmt_untyped(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(name) => write!(f, "{}", name),
            Self::Number(n) => write!(f, "{}", n),
            Self::Length(l) => match l {
                ui::Val::Auto => write!(f, "auto"),
                ui::Val::Px(v) => write!(f, "{}px", v),
                ui::Val::Percent(v) => write!(f, "{}%", v),
                ui::Val::Vw(v) => write!(f, "{}vw", v),
                ui::Val::Vh(v) => write!(f, "{}vh", v),
                ui::Val::VMin(v) => write!(f, "{}vmin", v),
                ui::Val::VMax(v) => write!(f, "{}vmax", v),
            },
            Self::Color(c) => fmt::Display::fmt(&c, f),
            Self::Asset(_) => todo!(),
            Self::Var(name) => write!(f, "var(--{})", name),
            _ => {
                panic!("Call to fmt_untyped with Self::Constant");
            }
        }
    }
}

impl<T> StyleExpr<T>
where
    StyleExpr<T>: Coerce<T>,
{
    fn optimize(&mut self) {
        let opt = self.coerce();
        if let Some(val) = opt {
            *self = Self::Constant(val);
        }
    }
}

impl Coerce<i16> for StyleExpr<i16> {
    fn coerce(&self) -> Option<i16> {
        match self {
            Self::Constant(v) => Some(*v as i16),
            Self::Number(v) => Some(*v as i16),
            _ => None,
        }
    }
}

impl Coerce<u16> for StyleExpr<u16> {
    fn coerce(&self) -> Option<u16> {
        match self {
            Self::Constant(v) => Some(*v as u16),
            Self::Number(v) => Some(*v as u16),
            _ => None,
        }
    }
}

impl Coerce<i32> for StyleExpr<i32> {
    fn coerce(&self) -> Option<i32> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Number(v) => Some(*v as i32),
            _ => None,
        }
    }
}

impl Coerce<f32> for StyleExpr<f32> {
    fn coerce(&self) -> Option<f32> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Number(v) => Some(*v),
            _ => None,
        }
    }
}

impl Coerce<AssetRef> for StyleExpr<AssetRef> {
    fn coerce(&self) -> Option<AssetRef> {
        match self {
            Self::Constant(v) => Some(v.clone()),
            Self::Asset(c) => Some(c.clone()),
            _ => None,
        }
    }
}

impl Coerce<ColorValue> for StyleExpr<ColorValue> {
    fn coerce(&self) -> Option<ColorValue> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Color(c) => Some(*c),
            _ => None,
        }
    }
}

impl Coerce<ui::Val> for StyleExpr<ui::Val> {
    fn coerce(&self) -> Option<ui::Val> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Length(v) => Some(*v),
            Self::Number(v) => Some(ui::Val::Px(*v)),
            Self::Ident(v) if v == "auto" => Some(ui::Val::Auto),
            _ => None,
        }
    }
}

impl Coerce<ui::Display> for StyleExpr<ui::Display> {
    fn coerce(&self) -> Option<ui::Display> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "grid" => Some(ui::Display::Grid),
                "flex" => Some(ui::Display::Flex),
                "none" => Some(ui::Display::None),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::PositionType> for StyleExpr<ui::PositionType> {
    fn coerce(&self) -> Option<ui::PositionType> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "relative" => Some(ui::PositionType::Relative),
                "absolute" => Some(ui::PositionType::Absolute),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::OverflowAxis> for StyleExpr<ui::OverflowAxis> {
    fn coerce(&self) -> Option<ui::OverflowAxis> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "clip" => Some(ui::OverflowAxis::Clip),
                "visible" => Some(ui::OverflowAxis::Visible),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::Direction> for StyleExpr<ui::Direction> {
    fn coerce(&self) -> Option<ui::Direction> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "inherit" => Some(ui::Direction::Inherit),
                "ltr" => Some(ui::Direction::LeftToRight),
                "rtl" => Some(ui::Direction::RightToLeft),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::AlignItems> for StyleExpr<ui::AlignItems> {
    fn coerce(&self) -> Option<ui::AlignItems> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
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
            _ => None,
        }
    }
}

impl Coerce<ui::AlignContent> for StyleExpr<ui::AlignContent> {
    fn coerce(&self) -> Option<ui::AlignContent> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
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
            _ => None,
        }
    }
}

impl Coerce<ui::AlignSelf> for StyleExpr<ui::AlignSelf> {
    fn coerce(&self) -> Option<ui::AlignSelf> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
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
            _ => None,
        }
    }
}

impl Coerce<ui::JustifyItems> for StyleExpr<ui::JustifyItems> {
    fn coerce(&self) -> Option<ui::JustifyItems> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "default" => Some(ui::JustifyItems::Default),
                "start" => Some(ui::JustifyItems::Start),
                "end" => Some(ui::JustifyItems::End),
                "center" => Some(ui::JustifyItems::Center),
                "baseline" => Some(ui::JustifyItems::Baseline),
                "stretch" => Some(ui::JustifyItems::Stretch),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::JustifyContent> for StyleExpr<ui::JustifyContent> {
    fn coerce(&self) -> Option<ui::JustifyContent> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
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
            _ => None,
        }
    }
}

impl Coerce<ui::JustifySelf> for StyleExpr<ui::JustifySelf> {
    fn coerce(&self) -> Option<ui::JustifySelf> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "auto" => Some(ui::JustifySelf::Auto),
                "start" => Some(ui::JustifySelf::Start),
                "end" => Some(ui::JustifySelf::End),
                "center" => Some(ui::JustifySelf::Center),
                "baseline" => Some(ui::JustifySelf::Baseline),
                "stretch" => Some(ui::JustifySelf::Stretch),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::FlexDirection> for StyleExpr<ui::FlexDirection> {
    fn coerce(&self) -> Option<ui::FlexDirection> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "row" => Some(ui::FlexDirection::Row),
                "row-reverse" => Some(ui::FlexDirection::RowReverse),
                "column" => Some(ui::FlexDirection::Column),
                "column-reverse" => Some(ui::FlexDirection::ColumnReverse),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::FlexWrap> for StyleExpr<ui::FlexWrap> {
    fn coerce(&self) -> Option<ui::FlexWrap> {
        match self {
            Self::Constant(v) => Some(*v),
            Self::Ident(ref n) => match n.as_str() {
                "nowrap" => Some(ui::FlexWrap::NoWrap),
                "wrap" => Some(ui::FlexWrap::Wrap),
                "wrap-reverse" => Some(ui::FlexWrap::WrapReverse),
                _ => None,
            },
            _ => None,
        }
    }
}

// Convert from an untyped expression.
impl<T> From<UntypedExpr> for StyleExpr<T> {
    fn from(value: UntypedExpr) -> Self {
        match value {
            UntypedExpr::Ident(v) => Self::Ident(v),
            UntypedExpr::Number(v) => Self::Number(v),
            UntypedExpr::Length(v) => Self::Length(v),
            UntypedExpr::Color(v) => Self::Color(v),
            UntypedExpr::Asset(v) => Self::Asset(v),
            UntypedExpr::Var(v) => Self::Var(v),
        }
    }
}

impl<T> std::str::FromStr for StyleExpr<T> {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        UntypedExpr::from_str(input).map(|expr| Self::from(expr))
    }
}

impl fmt::Display for StyleExpr<i16> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(n) => write!(f, "{}", n),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<u16> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(n) => write!(f, "{}", n),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<i32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(n) => write!(f, "{}", n),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<f32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(n) => write!(f, "{}", n),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<AssetRef> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(asset) => write!(f, "asset({})", asset.resolved()),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ColorValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(n) => write!(f, "{}", n),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::Val> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(n) => Self::Length(*n).fmt_untyped(f),
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::Display> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::Display::Flex => write!(f, "flex"),
                ui::Display::Grid => write!(f, "grid"),
                ui::Display::None => write!(f, "none"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::PositionType> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::PositionType::Relative => write!(f, "relative"),
                ui::PositionType::Absolute => write!(f, "absolute"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::OverflowAxis> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::OverflowAxis::Clip => write!(f, "clip"),
                ui::OverflowAxis::Visible => write!(f, "visible"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::Direction> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::Direction::Inherit => write!(f, "inherit"),
                ui::Direction::LeftToRight => write!(f, "ltr"),
                ui::Direction::RightToLeft => write!(f, "rtl"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::AlignItems> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::AlignItems::Default => write!(f, "default"),
                ui::AlignItems::Start => write!(f, "start"),
                ui::AlignItems::End => write!(f, "end"),
                ui::AlignItems::FlexStart => write!(f, "flex-start"),
                ui::AlignItems::FlexEnd => write!(f, "flex-end"),
                ui::AlignItems::Center => write!(f, "center"),
                ui::AlignItems::Baseline => write!(f, "baseline"),
                ui::AlignItems::Stretch => write!(f, "stretch"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::AlignContent> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
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
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::AlignSelf> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::AlignSelf::Auto => write!(f, "auto"),
                ui::AlignSelf::Start => write!(f, "start"),
                ui::AlignSelf::End => write!(f, "end"),
                ui::AlignSelf::FlexStart => write!(f, "flex-start"),
                ui::AlignSelf::FlexEnd => write!(f, "flex-end"),
                ui::AlignSelf::Center => write!(f, "center"),
                ui::AlignSelf::Baseline => write!(f, "baseline"),
                ui::AlignSelf::Stretch => write!(f, "stretch"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::JustifyItems> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::JustifyItems::Default => write!(f, "default"),
                ui::JustifyItems::Start => write!(f, "start"),
                ui::JustifyItems::End => write!(f, "end"),
                ui::JustifyItems::Center => write!(f, "center"),
                ui::JustifyItems::Baseline => write!(f, "baseline"),
                ui::JustifyItems::Stretch => write!(f, "stretch"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::JustifyContent> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
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
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::JustifySelf> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::JustifySelf::Auto => write!(f, "auto"),
                ui::JustifySelf::Start => write!(f, "start"),
                ui::JustifySelf::End => write!(f, "end"),
                ui::JustifySelf::Center => write!(f, "center"),
                ui::JustifySelf::Baseline => write!(f, "baseline"),
                ui::JustifySelf::Stretch => write!(f, "stretch"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

impl fmt::Display for StyleExpr<ui::FlexDirection> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::FlexDirection::Row => write!(f, "row"),
                ui::FlexDirection::RowReverse => write!(f, "row-reverse"),
                ui::FlexDirection::Column => write!(f, "column"),
                ui::FlexDirection::ColumnReverse => write!(f, "column-reverse"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}
impl fmt::Display for StyleExpr<ui::FlexWrap> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(val) => match val {
                ui::FlexWrap::NoWrap => write!(f, "nowrap"),
                ui::FlexWrap::Wrap => write!(f, "wrap"),
                ui::FlexWrap::WrapReverse => write!(f, "wrap-reverse"),
            },
            _ => self.fmt_untyped(f),
        }
    }
}

trait Ser<T>
where
    StyleExpr<T>: fmt::Display,
{
    fn serialize<S>(expr: StyleExpr<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match expr {
            StyleExpr::Number(n) => {
                if n.round() == n {
                    serializer.serialize_i32(n as i32)
                } else {
                    serializer.serialize_f32(n)
                }
            }
            StyleExpr::Length(ui::Val::Px(n)) => {
                if n.round() == n {
                    serializer.serialize_i32(n as i32)
                } else {
                    serializer.serialize_f32(n)
                }
            }
            _ => serializer.collect_str(&expr),
        }
    }
}

trait SerializeHelper
where
    Self: fmt::Display,
{
    fn serialize_const<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl SerializeHelper for StyleExpr<i16> {
    fn serialize_const<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let StyleExpr::Constant(n) = self {
            serializer.serialize_i16(*n)
        } else {
            panic!("Invalid serialization")
        }
    }
}

impl SerializeHelper for StyleExpr<u16> {
    fn serialize_const<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let StyleExpr::Constant(n) = self {
            serializer.serialize_u16(*n)
        } else {
            panic!("Invalid serialization")
        }
    }
}

impl SerializeHelper for StyleExpr<i32> {
    fn serialize_const<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let StyleExpr::Constant(n) = self {
            serializer.serialize_i32(*n)
        } else {
            panic!("Invalid serialization")
        }
    }
}

impl SerializeHelper for StyleExpr<f32> {
    fn serialize_const<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let StyleExpr::Constant(n) = self {
            if n.round() == *n {
                serializer.serialize_i32(*n as i32)
            } else {
                serializer.serialize_f32(*n)
            }
        } else {
            panic!("Invalid serialization")
        }
    }
}

impl SerializeHelper for StyleExpr<ui::Val> {
    fn serialize_const<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            StyleExpr::Constant(ui::Val::Px(n)) => {
                if n.round() == *n {
                    serializer.serialize_i32(*n as i32)
                } else {
                    serializer.serialize_f32(*n)
                }
            }
            StyleExpr::Constant(_) => serializer.collect_str(&self),
            _ => {
                panic!("Invalid serialization")
            }
        }
    }
}

impl SerializeHelper for StyleExpr<AssetRef> {}
impl SerializeHelper for StyleExpr<ColorValue> {}
impl SerializeHelper for StyleExpr<ui::Display> {}
impl SerializeHelper for StyleExpr<ui::PositionType> {}
impl SerializeHelper for StyleExpr<ui::OverflowAxis> {}
impl SerializeHelper for StyleExpr<ui::Direction> {}
impl SerializeHelper for StyleExpr<ui::AlignItems> {}
impl SerializeHelper for StyleExpr<ui::AlignContent> {}
impl SerializeHelper for StyleExpr<ui::AlignSelf> {}
impl SerializeHelper for StyleExpr<ui::JustifyItems> {}
impl SerializeHelper for StyleExpr<ui::JustifyContent> {}
impl SerializeHelper for StyleExpr<ui::JustifySelf> {}
impl SerializeHelper for StyleExpr<ui::FlexWrap> {}
impl SerializeHelper for StyleExpr<ui::FlexDirection> {}

impl<T> Serialize for StyleExpr<T>
where
    StyleExpr<T>: fmt::Display,
    StyleExpr<T>: SerializeHelper,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Constant(_) => self.serialize_const(serializer),
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

impl<'de, T> Deserialize<'de> for StyleExpr<T>
where
    StyleExpr<T>: Coerce<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        UntypedExpr::deserialize(deserializer).map(|expr| {
            let mut val = Self::from(expr);
            val.optimize();
            val
        })
        // deserializer.deserialize_any(ExprVisitor)
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Color;

    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_trailing_space() {
        assert_eq!(
            StyleExpr::<f32>::from_str("#f00 ").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::RED))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("1 ").unwrap(),
            StyleExpr::Number(1.)
        );
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(
            StyleExpr::<f32>::from_str("#f00").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::RED))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("#00f").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::BLUE))
        );
        // Invalid color value parsed as NONE
        assert_eq!(
            StyleExpr::<f32>::from_str("#0f").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::NONE))
        );
    }

    #[test]
    fn test_parse_color_fn() {
        assert_eq!(
            StyleExpr::<f32>::from_str("rgba( 255 255 255 )").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 1.)))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("rgba(255, 255, 255)").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 1.)))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("rgba(255, 255, 255, 0.5)").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("rgba(255 255 255 / 0.5)").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("rgb(255 255 255 / 0.5)").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::rgba(1., 1., 1., 0.5)))
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("hsla(360 100 100 / 0.5)").unwrap(),
            StyleExpr::Color(ColorValue::Color(Color::hsla(1., 1., 1., 0.5)))
        );
    }

    #[test]
    fn test_parse_int() {
        assert_eq!(
            StyleExpr::<f32>::from_str("1").unwrap(),
            StyleExpr::Number(1.)
        );
        assert_eq!(
            StyleExpr::<f32>::from_str("77").unwrap(),
            StyleExpr::Number(77.)
        );
    }

    #[test]
    fn test_parse_asset() {
        assert_eq!(
            StyleExpr::<f32>::from_str("asset(../image.png)").unwrap(),
            StyleExpr::Asset(AssetRef::new("../image.png"))
        );
    }
}
