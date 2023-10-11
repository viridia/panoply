use std::{fmt, sync::Arc};

use bevy::{asset::AssetPath, prelude::*, ui, utils::HashMap};

use super::{coerce::Coerce, ElementStyle, Renderable};

/// Defines the types of parameters that can be passed to a template.
#[derive(Debug, Clone)]
pub struct Template {
    pub params: HashMap<String, TemplateParam>,
    pub expr: Expr,
}

/// Defines the types of parameters that can be passed to a template.
#[derive(Debug, Clone)]
pub struct TemplateParam {
    pub param_type: String,
}

impl TemplateParam {
    pub fn new(ty: &str) -> Self {
        Self {
            param_type: ty.to_string(),
        }
    }
}

/// An expression which represents a parsed value.
#[derive(Debug, Clone)]
pub enum Expr {
    /// No expression
    Null,

    /// A boolean expression
    Bool(bool),

    /// An identifier
    Ident(String),

    /// A text string
    Text(String),

    /// A floating-point number
    Number(f32),

    /// A length, such as "2px"
    Length(ui::Val),

    /// A list of expressions
    List(Box<[Expr]>),

    /// A color value
    Color(Color),

    /// A reference to a renderable UI component
    Renderable(Arc<dyn Renderable>),

    /// A reference to a style asset
    Style(Arc<ElementStyle>),

    /// A reference to an asset: "$(path)"
    Asset(AssetPath<'static>),

    /// A reference to a named variable "${varname}".
    Var(String),

    /// A template declaration.
    Template(Box<Template>),
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Ident(l0), Self::Ident(r0)) => l0 == r0,
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::Length(l0), Self::Length(r0)) => l0 == r0,
            (Self::List(l0), Self::List(r0)) => l0 == r0,
            (Self::Color(l0), Self::Color(r0)) => l0 == r0,
            (Self::Renderable(l0), Self::Renderable(r0)) => std::ptr::eq(l0.as_ref(), r0.as_ref()),
            (Self::Style(l0), Self::Style(r0)) => std::ptr::eq(l0.as_ref(), r0.as_ref()),
            (Self::Asset(l0), Self::Asset(r0)) => l0 == r0,
            (Self::Var(l0), Self::Var(r0)) => l0 == r0,
            (Self::Template(l0), Self::Template(r0)) => std::ptr::eq(l0.as_ref(), r0.as_ref()),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Default for Expr {
    fn default() -> Self {
        Expr::Null
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(true) => write!(f, "true"),
            Self::Bool(false) => write!(f, "false"),
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
            Self::Color(_) => todo!(),
            Self::Asset(_) => todo!(),
            Self::List(l) => {
                write!(f, "[")?;
                for elt in l.iter() {
                    elt.fmt(f)?;
                    write!(f, ",")?;
                }
                write!(f, "]")
            }
            Self::Var(name) => write!(f, "${{{}}}", name),
            Self::Template(tmp) => write!(f, "template {}", tmp.expr),
            _ => todo!(),
        }
    }
}

impl Coerce<u16> for Expr {
    fn coerce(&self) -> Option<u16> {
        match self {
            Self::Number(v) => Some(*v as u16),
            _ => None,
        }
    }
}

impl Coerce<i16> for Expr {
    fn coerce(&self) -> Option<i16> {
        match self {
            Self::Number(v) => Some(*v as i16),
            _ => None,
        }
    }
}

impl Coerce<i32> for Expr {
    fn coerce(&self) -> Option<i32> {
        match self {
            Self::Number(v) => Some(*v as i32),
            _ => None,
        }
    }
}

impl Coerce<f32> for Expr {
    fn coerce(&self) -> Option<f32> {
        match self {
            Self::Number(v) => Some(*v),
            _ => None,
        }
    }
}

impl Coerce<Color> for Expr {
    fn coerce(&self) -> Option<Color> {
        match self {
            Self::Color(c) => Some(*c),
            _ => None,
        }
    }
}

impl Coerce<Option<Color>> for Expr {
    fn coerce(&self) -> Option<Option<Color>> {
        match self {
            Self::Color(c) => Some(Some(*c)),
            _ => None,
        }
    }
}

impl Coerce<ui::Val> for Expr {
    fn coerce(&self) -> Option<ui::Val> {
        match self {
            Self::Length(v) => Some(*v),
            Self::Number(v) => Some(ui::Val::Px(*v)),
            Self::Ident(v) if v == "auto" => Some(ui::Val::Auto),
            _ => None,
        }
    }
}

impl Coerce<ui::Display> for Expr {
    fn coerce(&self) -> Option<ui::Display> {
        match self {
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

impl Coerce<ui::PositionType> for Expr {
    fn coerce(&self) -> Option<ui::PositionType> {
        match self {
            Self::Ident(ref n) => match n.as_str() {
                "relative" => Some(ui::PositionType::Relative),
                "absolute" => Some(ui::PositionType::Absolute),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::OverflowAxis> for Expr {
    fn coerce(&self) -> Option<ui::OverflowAxis> {
        match self {
            Self::Ident(ref n) => match n.as_str() {
                "clip" => Some(ui::OverflowAxis::Clip),
                "visible" => Some(ui::OverflowAxis::Visible),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Coerce<ui::Direction> for Expr {
    fn coerce(&self) -> Option<ui::Direction> {
        match self {
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

impl Coerce<ui::AlignItems> for Expr {
    fn coerce(&self) -> Option<ui::AlignItems> {
        match self {
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

impl Coerce<ui::AlignContent> for Expr {
    fn coerce(&self) -> Option<ui::AlignContent> {
        match self {
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

impl Coerce<ui::AlignSelf> for Expr {
    fn coerce(&self) -> Option<ui::AlignSelf> {
        match self {
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

impl Coerce<ui::JustifyItems> for Expr {
    fn coerce(&self) -> Option<ui::JustifyItems> {
        match self {
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

impl Coerce<ui::JustifyContent> for Expr {
    fn coerce(&self) -> Option<ui::JustifyContent> {
        match self {
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

impl Coerce<ui::JustifySelf> for Expr {
    fn coerce(&self) -> Option<ui::JustifySelf> {
        match self {
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

impl Coerce<ui::FlexDirection> for Expr {
    fn coerce(&self) -> Option<ui::FlexDirection> {
        match self {
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

impl Coerce<ui::FlexWrap> for Expr {
    fn coerce(&self) -> Option<ui::FlexWrap> {
        match self {
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
