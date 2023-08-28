use std::{borrow::Cow, fmt, marker::PhantomData};

use crate::guise::style::{
    expr::{Expr, TypeHint},
    expr_list::ExprList,
};

use super::{Selector, StyleAttr};
use bevy::utils::HashMap;
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};

/// A collection of style attributes which can be merged to create a `ComputedStyle`.
#[derive(Debug, Default, Clone)]
pub struct Style<'a> {
    /// List of style attributes.
    /// Rather than storing the attributes in a struct full of optional fields, we store a flat
    /// vector of enums, each of which stores a single style attribute. This "sparse" representation
    /// allows for fast merging of styles, particularly for styles which have few or no attributes.
    attrs: Vec<StyleAttr>,

    /// List of style variables to define when this style is invoked.
    vars: HashMap<Cow<'a, str>, Expr>,

    /// List of conditional styles
    selectors: Vec<(Selector, Box<Style<'a>>)>,
}

impl<'a> Style<'a> {
    pub fn new() -> Self {
        Self {
            attrs: Vec::new(),
            vars: HashMap::new(),
            selectors: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            attrs: Vec::with_capacity(capacity),
            vars: HashMap::new(),
            selectors: Vec::new(),
        }
    }

    /// Construct a new `StyleMap` from a list of `StyleAttr`s.
    pub fn from_attrs(attrs: &[StyleAttr]) -> Self {
        Self {
            attrs: Vec::from(attrs),
            vars: HashMap::new(),
            selectors: Vec::new(),
        }
    }

    /// Number of style attributes in the map.
    pub fn len_attrs(&self) -> usize {
        self.attrs.len()
    }
}

impl<'a> Serialize for Style<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut st = serializer.serialize_struct("StyleMap", self.attrs.len())?;
        for attr in self.attrs.iter() {
            match attr {
                StyleAttr::BackgroundColor(val) => st.serialize_field("background-color", val)?,
                StyleAttr::BorderColor(val) => st.serialize_field("border-color", val)?,
                StyleAttr::Color(val) => st.serialize_field("color", val)?,
                StyleAttr::ZIndex(val) => st.serialize_field("z-index", val)?,

                StyleAttr::Display(val) => st.serialize_field("display", val)?,

                StyleAttr::Left(val) => st.serialize_field("left", val)?,
                StyleAttr::Right(val) => st.serialize_field("right", val)?,
                StyleAttr::Top(val) => st.serialize_field("top", val)?,
                StyleAttr::Bottom(val) => st.serialize_field("bottom", val)?,

                StyleAttr::Width(val) => st.serialize_field("width", val)?,
                StyleAttr::Height(val) => st.serialize_field("height", val)?,
                StyleAttr::MinWidth(val) => st.serialize_field("min-width", val)?,
                StyleAttr::MinHeight(val) => st.serialize_field("min-height", val)?,
                StyleAttr::MaxWidth(val) => st.serialize_field("max-width", val)?,
                StyleAttr::MaxHeight(val) => st.serialize_field("max-height", val)?,

                StyleAttr::Margin(val) => st.serialize_field("margin", val)?,
                StyleAttr::MarginLeft(val) => st.serialize_field("margin-left", val)?,
                StyleAttr::MarginRight(val) => st.serialize_field("margin-right", val)?,
                StyleAttr::MarginTop(val) => st.serialize_field("margin-top", val)?,
                StyleAttr::MarginBottom(val) => st.serialize_field("margin-bottom", val)?,

                StyleAttr::Padding(val) => st.serialize_field("padding", val)?,
                StyleAttr::PaddingLeft(val) => st.serialize_field("padding-left", val)?,
                StyleAttr::PaddingRight(val) => st.serialize_field("padding-right", val)?,
                StyleAttr::PaddingTop(val) => st.serialize_field("padding-top", val)?,
                StyleAttr::PaddingBottom(val) => st.serialize_field("padding-bottom", val)?,

                StyleAttr::Border(val) => st.serialize_field("border", val)?,
                StyleAttr::BorderLeft(val) => st.serialize_field("border-left", val)?,
                StyleAttr::BorderRight(val) => st.serialize_field("border-right", val)?,
                StyleAttr::BorderTop(val) => st.serialize_field("border-top", val)?,
                StyleAttr::BorderBottom(val) => st.serialize_field("border-bottom", val)?,

                StyleAttr::FlexGrow(val) => st.serialize_field("flex-grow", val)?,
                StyleAttr::FlexShrink(val) => st.serialize_field("flex-shrink", val)?,
                _ => todo!("Implement serialization for {:?}", attr),
            };
        }
        st.end()
    }
}

const FIELDS: &'static [&'static str] = &[
    // Colors
    "background-color",
    "border-color",
    "color",
    // Positioning
    "z-index",
    "display",
    // Rect
    "left",
    "right",
    "top",
    "bottom",
    // Size
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    // Margins
    "margin",
    "margin-left",
    "margin-right",
    "margin-top",
    "margin-bottom",
    // Padding
    "padding",
    "padding-left",
    "padding-right",
    "padding-top",
    "padding-bottom",
    // Border
    "border",
    "border-left",
    "border-right",
    "border-top",
    "border-bottom",
    // Flex
    "flex-grow",
    "flex-shrink",
    "vars",
    "selectors",
];

impl<'de, 'a> Deserialize<'de> for Style<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        enum Field {
            BackgroundColor,
            BorderColor,
            Color,

            ZIndex,
            Display,
            // Position(bevy::ui::PositionType),
            // Overflow(bevy::ui::OverflowAxis),
            // OverflowX(bevy::ui::OverflowAxis),
            // OverflowY(bevy::ui::OverflowAxis),
            // Direction(bevy::ui::Direction),
            Left,
            Right,
            Top,
            Bottom,

            Width,
            Height,
            MinWidth,
            MinHeight,
            MaxWidth,
            MaxHeight,

            // // pub aspect_ratio: StyleProp<f32>,
            // AlignItems(bevy::ui::AlignItems),
            // JustifyItems(bevy::ui::JustifyItems),
            // AlignSelf(bevy::ui::AlignSelf),
            // JustifySelf(bevy::ui::JustifySelf),
            // AlignContent(bevy::ui::AlignContent),
            // JustifyContent(bevy::ui::JustifyContent),
            Margin,
            MarginLeft,
            MarginRight,
            MarginTop,
            MarginBottom,

            Padding,
            PaddingLeft,
            PaddingRight,
            PaddingTop,
            PaddingBottom,

            Border,
            BorderLeft,
            BorderRight,
            BorderTop,
            BorderBottom,

            // FlexDirection(bevy::ui::FlexDirection),
            // FlexWrap(bevy::ui::FlexWrap),
            FlexGrow,
            FlexShrink,
            // FlexBasis(bevy::ui::Val),

            // RowGap(bevy::ui::Val),
            // ColumnGap(bevy::ui::Val),
            // Gap(bevy::ui::Val),

            // // TODO:
            // GridAutoFlow(bevy::ui::GridAutoFlow),
            // // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
            // // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
            // // pub grid_auto_rows: Option<Vec<GridTrack>>,
            // // pub grid_auto_columns: Option<Vec<GridTrack>>,
            // GridRow(bevy::ui::GridPlacement),
            // GridRowStart(i16),
            // GridRowSpan(u16),
            // GridRowEnd(i16),
            // GridColumn(bevy::ui::GridPlacement),
            // GridColumnStart(i16),
            // GridColumnSpan(u16),
            // GridColumnEnd(i16),

            // LineBreak(BreakLineOn),
            Vars,
            Selectors,
        }

        struct StyleMapVisitor;

        impl<'de> Visitor<'de> for StyleMapVisitor {
            type Value = Style<'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("style definition")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                type SA = StyleAttr;
                let mut st = Style::with_capacity(map.size_hint().unwrap_or(0));
                let attrs = &mut st.attrs;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::BackgroundColor => {
                            attrs.push(SA::BackgroundColor(map.next_value::<Expr>()?))
                        }
                        Field::BorderColor => {
                            attrs.push(SA::BorderColor(map.next_value::<Expr>()?))
                        }
                        Field::Color => attrs.push(SA::Color(map.next_value::<Expr>()?)),
                        Field::ZIndex => attrs.push(SA::ZIndex(map.next_value::<Expr>()?)),

                        Field::Display => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Display);
                            attrs.push(SA::Display(val))
                        }

                        Field::Left => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Left(val))
                        }

                        Field::Right => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Right(val))
                        }

                        Field::Top => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Top(val))
                        }

                        Field::Bottom => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Bottom(val))
                        }

                        Field::Width => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Width(val))
                        }

                        Field::Height => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Height(val))
                        }

                        Field::MinWidth => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MinWidth(val))
                        }

                        Field::MinHeight => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MinHeight(val))
                        }

                        Field::MaxWidth => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MaxWidth(val))
                        }

                        Field::MaxHeight => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MaxHeight(val))
                        }

                        Field::Margin => {
                            let mut val = map.next_value::<ExprList>()?.to_expr();
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Margin(val))
                        }
                        Field::MarginLeft => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MarginLeft(val))
                        }
                        Field::MarginRight => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MarginRight(val))
                        }
                        Field::MarginTop => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MarginTop(val))
                        }
                        Field::MarginBottom => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::MarginBottom(val))
                        }

                        Field::Padding => {
                            let mut val = map.next_value::<ExprList>()?.to_expr();
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Padding(val))
                        }
                        Field::PaddingLeft => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::PaddingLeft(val))
                        }
                        Field::PaddingRight => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::PaddingRight(val))
                        }
                        Field::PaddingTop => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::PaddingTop(val))
                        }
                        Field::PaddingBottom => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::PaddingBottom(val))
                        }

                        Field::Border => {
                            let mut val = map.next_value::<ExprList>()?.to_expr();
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::Border(val))
                        }
                        Field::BorderLeft => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::BorderLeft(val))
                        }
                        Field::BorderRight => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::BorderRight(val))
                        }
                        Field::BorderTop => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::BorderTop(val))
                        }
                        Field::BorderBottom => {
                            let mut val = map.next_value::<Expr>()?;
                            val.optimize(TypeHint::Length);
                            attrs.push(SA::BorderBottom(val))
                        }

                        Field::FlexGrow => attrs.push(SA::FlexGrow(map.next_value::<Expr>()?)),
                        Field::FlexShrink => attrs.push(SA::FlexShrink(map.next_value::<Expr>()?)),
                        Field::Vars => {
                            st.vars = map.next_value::<VarsMapSer>()?.0;
                        }
                        Field::Selectors => {
                            todo!()
                        }
                    }
                }
                Ok(st)
            }
        }

        deserializer.deserialize_struct("StyleMap", FIELDS, StyleMapVisitor)
    }
}

struct VarsMapSer<'a>(HashMap<Cow<'a, str>, Expr>);

impl<'de, 'a> Deserialize<'de> for VarsMapSer<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(VarsMapVisitor::<'a> {
            marker: &PhantomData,
        })
    }
}

struct VarsMapVisitor<'a> {
    marker: &'a PhantomData<()>,
}

impl<'de, 'a> Visitor<'de> for VarsMapVisitor<'a> {
    type Value = VarsMapSer<'a>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("style definition")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
        A::Error: serde::de::Error,
    {
        let mut result: HashMap<Cow<'a, str>, Expr> =
            HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let expr = map.next_value::<Expr>()?;
            if key.starts_with("--") {
                result.insert(key[2..].to_owned().into(), expr);
            } else {
                return Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Str(&key),
                    &"Expression list",
                ));
            }
        }
        Ok(VarsMapSer(result))
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui;

    use super::*;

    #[test]
    fn test_serialize_misc_props() {
        let map = Style::from_attrs(&[
            StyleAttr::ZIndex(Expr::Number(7.)),
            StyleAttr::FlexGrow(Expr::Number(2.)),
            StyleAttr::FlexShrink(Expr::Number(3.)),
        ]);
        let ser = serde_json::to_string(&map);
        assert_eq!(
            ser.unwrap(),
            r#"{"z-index":7,"flex-grow":2,"flex-shrink":3}"#
        );
    }

    #[test]
    fn test_deserialize_basic_prop() {
        let des =
            serde_json::from_str::<Style>(r#"{"background-color":"rgba(255, 0, 0, 1)"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"background-color":"rgba(255, 0, 0, 1)"}"#);
    }

    #[test]
    fn test_deserialize_misc_props() {
        let des =
            serde_json::from_str::<Style>(r#"{"z-index":7,"flex-grow":2.0,"flex-shrink":3.1}"#)
                .unwrap();
        assert_eq!(des.attrs.len(), 3);
        let ser = serde_json::to_string(&des);
        assert_eq!(
            ser.unwrap(),
            r#"{"z-index":7,"flex-grow":2,"flex-shrink":3.1}"#
        );
    }

    #[test]
    fn test_deserialize_length_no_unit() {
        let des = serde_json::from_str::<Style>(r#"{"right":7}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(des.attrs[0], StyleAttr::Right(Expr::Number(7.)));
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"right":7}"#);
    }

    #[test]
    fn test_deserialize_length_px() {
        let des = serde_json::from_str::<Style>(r#"{"right":"7px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Right(Expr::Length(ui::Val::Px(7.)))
        );
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"right":7}"#);
    }

    #[test]
    fn test_deserialize_auto() {
        let des = serde_json::from_str::<Style>(r#"{"right":"auto"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(des.attrs[0], StyleAttr::Right(Expr::Length(ui::Val::Auto)));
    }

    #[test]
    fn test_serialize_display() {
        let map = Style::from_attrs(&[StyleAttr::Display(Expr::Ident("grid".to_string()))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"display":"grid"}"#);

        let map = Style::from_attrs(&[StyleAttr::Display(Expr::Display(ui::Display::Grid))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"display":"grid"}"#);
    }

    #[test]
    fn test_deserialize_display() {
        let des = serde_json::from_str::<Style>(r#"{"display":"grid"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Display(Expr::Display(ui::Display::Grid))
        );
    }

    #[test]
    fn test_serialize_uirect() {
        let map = Style::from_attrs(&[StyleAttr::Margin(Expr::List(vec![Expr::Number(0.)]))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"margin":"0"}"#);

        let map = Style::from_attrs(&[StyleAttr::Margin(Expr::List(vec![
            Expr::Number(0.),
            Expr::Number(0.),
        ]))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"margin":"0 0"}"#);

        let map = Style::from_attrs(&[StyleAttr::Margin(Expr::List(vec![
            Expr::Length(ui::Val::Auto),
            Expr::Length(ui::Val::Px(7.)),
        ]))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"margin":"auto 7px"}"#);
    }

    #[test]
    fn test_deserialize_uirect() {
        // Unitless number
        let des = serde_json::from_str::<Style>(r#"{"margin":0}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(Expr::List(vec![Expr::Number(0.)]))
        );

        // Unitless string
        let des = serde_json::from_str::<Style>(r#"{"margin":"0"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(Expr::List(vec![Expr::Number(0.)]))
        );

        // Pixel units
        let des = serde_json::from_str::<Style>(r#"{"margin":"0px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(Expr::List(vec![Expr::Length(ui::Val::Px(0.))]))
        );

        // Multiple values
        let des = serde_json::from_str::<Style>(r#"{"margin":"0px 0px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(Expr::List(vec![
                Expr::Length(ui::Val::Px(0.)),
                Expr::Length(ui::Val::Px(0.))
            ]))
        );

        // Optimize ident to 'auto'
        let des = serde_json::from_str::<Style>(r#"{"margin":"0px auto"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(Expr::List(vec![
                Expr::Length(ui::Val::Px(0.)),
                Expr::Length(ui::Val::Auto)
            ]))
        );
    }

    #[test]
    fn test_deserialize_vars() {
        let des = serde_json::from_str::<Style>(r#"{"vars":{}}"#).unwrap();
        assert_eq!(des.vars.len(), 0);

        let des = serde_json::from_str::<Style>(r#"{"vars":{"--x":1}}"#).unwrap();
        assert_eq!(des.vars.len(), 1);
        assert_eq!(des.vars.get("x").unwrap(), &Expr::Number(1.));

        let des = serde_json::from_str::<Style>(r#"{"vars":{"--bg":"auto"}}"#).unwrap();
        assert_eq!(des.vars.len(), 1);
        assert_eq!(
            des.vars.get("bg").unwrap(),
            &Expr::Ident("auto".to_string())
        );
    }
}
