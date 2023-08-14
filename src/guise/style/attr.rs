use bevy::{prelude::Color, ui::*};
use lazy_static::lazy_static;
use quick_xml::events::BytesStart;
use regex::Regex;
use std::str::FromStr;

use crate::guise::GuiseError;

use super::ComputedStyle;

/** Set of style attributes that can be applied to construct a style. */
#[derive(Debug, Clone)]
pub enum StyleAttr {
    BackgroundColor(Color),
    BorderColor(Color),

    Display(bevy::ui::Display),
    Position(bevy::ui::PositionType),
    Overflow(bevy::ui::OverflowAxis),
    OverflowX(bevy::ui::OverflowAxis),
    OverflowY(bevy::ui::OverflowAxis),
    Direction(bevy::ui::Direction),

    Left(bevy::ui::Val),
    Right(bevy::ui::Val),
    Top(bevy::ui::Val),
    Bottom(bevy::ui::Val),

    Width(bevy::ui::Val),
    Height(bevy::ui::Val),
    MinWidth(bevy::ui::Val),
    MinHeight(bevy::ui::Val),
    MaxWidth(bevy::ui::Val),
    MaxHeight(bevy::ui::Val),

    // pub aspect_ratio: StyleProp<f32>,
    AlignItems(bevy::ui::AlignItems),
    JustifyItems(bevy::ui::JustifyItems),
    AlignSelf(bevy::ui::AlignSelf),
    JustifySelf(bevy::ui::JustifySelf),

    // Allow margin sides to be set individually
    Margin(bevy::ui::UiRect),
    MarginLeft(bevy::ui::Val),
    MarginRight(bevy::ui::Val),
    MarginTop(bevy::ui::Val),
    MarginBottom(bevy::ui::Val),

    Padding(bevy::ui::UiRect),
    PaddingLeft(bevy::ui::Val),
    PaddingRight(bevy::ui::Val),
    PaddingTop(bevy::ui::Val),
    PaddingBottom(bevy::ui::Val),

    Border(bevy::ui::UiRect),
    BorderLeft(bevy::ui::Val),
    BorderRight(bevy::ui::Val),
    BorderTop(bevy::ui::Val),
    BorderBottom(bevy::ui::Val),

    FlexDirection(bevy::ui::FlexDirection),
    FlexWrap(bevy::ui::FlexWrap),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(bevy::ui::Val),

    RowGap(bevy::ui::Val),
    ColumnGap(bevy::ui::Val),
    Gap(bevy::ui::Val),
    // TODO:
    // pub grid_auto_flow: StyleProp<GridAutoFlow>,
    // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
    // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
    // pub grid_auto_rows: Option<Vec<GridTrack>>,
    // pub grid_auto_columns: Option<Vec<GridTrack>>,
    // pub grid_row: GridPlacement,
    // pub grid_column: GridPlacement,
}

impl StyleAttr {
    /// Apply this style attribute to a computed style.
    pub fn apply(&self, computed: &mut ComputedStyle) {
        match self {
            StyleAttr::BackgroundColor(val) => {
                computed.background_color = Some(*val);
            }
            StyleAttr::BorderColor(val) => {
                computed.border_color = Some(*val);
            }

            StyleAttr::Display(val) => {
                computed.style.display = *val;
            }
            StyleAttr::Position(val) => {
                computed.style.position_type = *val;
            }
            StyleAttr::Overflow(val) => {
                computed.style.overflow.x = *val;
                computed.style.overflow.y = *val;
            }
            StyleAttr::OverflowX(val) => {
                computed.style.overflow.x = *val;
            }
            StyleAttr::OverflowY(val) => {
                computed.style.overflow.y = *val;
            }
            StyleAttr::Direction(val) => {
                computed.style.direction = *val;
            }

            StyleAttr::Left(val) => {
                computed.style.left = *val;
            }
            StyleAttr::Right(val) => {
                computed.style.right = *val;
            }
            StyleAttr::Top(val) => {
                computed.style.top = *val;
            }
            StyleAttr::Bottom(val) => {
                computed.style.bottom = *val;
            }

            StyleAttr::Width(val) => {
                computed.style.width = *val;
            }
            StyleAttr::Height(val) => {
                computed.style.height = *val;
            }
            StyleAttr::MinWidth(val) => {
                computed.style.min_width = *val;
            }
            StyleAttr::MinHeight(val) => {
                computed.style.min_height = *val;
            }
            StyleAttr::MaxWidth(val) => {
                computed.style.max_width = *val;
            }
            StyleAttr::MaxHeight(val) => {
                computed.style.max_height = *val;
            }

            StyleAttr::AlignItems(val) => {
                computed.style.align_items = *val;
            }
            StyleAttr::JustifyItems(val) => {
                computed.style.justify_items = *val;
            }
            StyleAttr::AlignSelf(val) => {
                computed.style.align_self = *val;
            }
            StyleAttr::JustifySelf(val) => {
                computed.style.justify_self = *val;
            }

            StyleAttr::Margin(val) => {
                computed.style.margin = *val;
            }
            StyleAttr::MarginLeft(val) => {
                computed.style.margin.left = *val;
            }
            StyleAttr::MarginRight(val) => {
                computed.style.margin.right = *val;
            }
            StyleAttr::MarginTop(val) => {
                computed.style.margin.top = *val;
            }
            StyleAttr::MarginBottom(val) => {
                computed.style.margin.bottom = *val;
            }

            StyleAttr::Padding(val) => {
                computed.style.padding = *val;
            }
            StyleAttr::PaddingLeft(val) => {
                computed.style.padding.left = *val;
            }
            StyleAttr::PaddingRight(val) => {
                computed.style.padding.right = *val;
            }
            StyleAttr::PaddingTop(val) => {
                computed.style.padding.top = *val;
            }
            StyleAttr::PaddingBottom(val) => {
                computed.style.padding.bottom = *val;
            }

            StyleAttr::Border(val) => {
                computed.style.border = *val;
            }
            StyleAttr::BorderLeft(val) => {
                computed.style.border.left = *val;
            }
            StyleAttr::BorderRight(val) => {
                computed.style.border.right = *val;
            }
            StyleAttr::BorderTop(val) => {
                computed.style.border.top = *val;
            }
            StyleAttr::BorderBottom(val) => {
                computed.style.border.bottom = *val;
            }

            StyleAttr::FlexDirection(val) => {
                computed.style.flex_direction = *val;
            }
            StyleAttr::FlexWrap(val) => {
                computed.style.flex_wrap = *val;
            }
            StyleAttr::FlexGrow(val) => {
                computed.style.flex_grow = *val;
            }
            StyleAttr::FlexShrink(val) => {
                computed.style.flex_shrink = *val;
            }
            StyleAttr::FlexBasis(val) => {
                computed.style.flex_basis = *val;
            }

            StyleAttr::RowGap(val) => {
                computed.style.row_gap = *val;
            }
            StyleAttr::ColumnGap(val) => {
                computed.style.column_gap = *val;
            }
            StyleAttr::Gap(val) => {
                computed.style.row_gap = *val;
                computed.style.column_gap = *val;
            }
        }
    }

    /// Parse a `StyleAttr` from an XML attribute name/value pair.
    pub fn parse<'a>(name: &'a [u8], value: &str) -> Result<Option<Self>, GuiseError> {
        Ok(Some(match name {
            b"background-color" => StyleAttr::BackgroundColor(StyleAttr::parse_color(value)?),

            b"border-color" => StyleAttr::BorderColor(StyleAttr::parse_color(value)?),

            b"display" => StyleAttr::Display(match value {
                "none" => Display::None,
                "grid" => Display::Grid,
                "flex" => Display::Flex,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"position" => StyleAttr::Position(match value {
                "absolute" => PositionType::Absolute,
                "relative" => PositionType::Relative,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"overflow" => StyleAttr::Overflow(match value {
                "clip" => OverflowAxis::Clip,
                "visible" => OverflowAxis::Visible,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"overflow-x" => StyleAttr::OverflowX(match value {
                "clip" => OverflowAxis::Clip,
                "visible" => OverflowAxis::Visible,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"overflow-y" => StyleAttr::OverflowY(match value {
                "clip" => OverflowAxis::Clip,
                "visible" => OverflowAxis::Visible,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            //     Direction,
            b"left" => StyleAttr::Left(StyleAttr::parse_val(value)?),
            b"right" => StyleAttr::Right(StyleAttr::parse_val(value)?),
            b"top" => StyleAttr::Top(StyleAttr::parse_val(value)?),
            b"bottom" => StyleAttr::Bottom(StyleAttr::parse_val(value)?),

            b"width" => StyleAttr::Width(StyleAttr::parse_val(value)?),
            b"height" => StyleAttr::Height(StyleAttr::parse_val(value)?),
            b"min-width" => StyleAttr::MinWidth(StyleAttr::parse_val(value)?),
            b"min-height" => StyleAttr::MinHeight(StyleAttr::parse_val(value)?),
            b"max-width" => StyleAttr::MaxWidth(StyleAttr::parse_val(value)?),
            b"max-height" => StyleAttr::MaxHeight(StyleAttr::parse_val(value)?),

            //     // pub aspect_ratio: StyleProp<f32>,
            //     AlignItems,
            //     JustifyItems,
            //     AlignSelf,
            //     JustifySelf,
            b"margin" => StyleAttr::Margin(StyleAttr::parse_uirect(value)?),
            b"margin-left" => StyleAttr::MarginLeft(StyleAttr::parse_val(value)?),
            b"margin-right" => StyleAttr::MarginRight(StyleAttr::parse_val(value)?),
            b"margin-top" => StyleAttr::MarginTop(StyleAttr::parse_val(value)?),
            b"margin-bottom" => StyleAttr::MarginBottom(StyleAttr::parse_val(value)?),

            b"padding" => StyleAttr::Padding(StyleAttr::parse_uirect(value)?),
            b"padding-left" => StyleAttr::PaddingLeft(StyleAttr::parse_val(value)?),
            b"padding-right" => StyleAttr::PaddingRight(StyleAttr::parse_val(value)?),
            b"padding-top" => StyleAttr::PaddingTop(StyleAttr::parse_val(value)?),
            b"padding-bottom" => StyleAttr::PaddingBottom(StyleAttr::parse_val(value)?),

            b"border" => StyleAttr::Border(StyleAttr::parse_uirect(value)?),
            b"border-left" => StyleAttr::BorderLeft(StyleAttr::parse_val(value)?),
            b"border-right" => StyleAttr::BorderRight(StyleAttr::parse_val(value)?),
            b"border-top" => StyleAttr::BorderTop(StyleAttr::parse_val(value)?),
            b"border-bottom" => StyleAttr::BorderBottom(StyleAttr::parse_val(value)?),

            b"flex-direction" => StyleAttr::FlexDirection(match value {
                "row" => FlexDirection::Row,
                "column" => FlexDirection::Column,
                "row-reverse" => FlexDirection::RowReverse,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            //     FlexWrap,
            //     FlexGrow,
            //     FlexShrink,
            //     FlexBasis,
            b"row-gap" => StyleAttr::RowGap(StyleAttr::parse_val(value)?),
            b"column-gap" => StyleAttr::ColumnGap(StyleAttr::parse_val(value)?),
            b"gap" => StyleAttr::Gap(StyleAttr::parse_val(value)?),

            //     // TODO:
            //     // pub grid_auto_flow: StyleProp<GridAutoFlow>,
            //     // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
            //     // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
            //     // pub grid_auto_rows: Option<Vec<GridTrack>>,
            //     // pub grid_auto_columns: Option<Vec<GridTrack>>,
            //     // pub grid_row: GridPlacement,
            //     // pub grid_column: GridPlacement,
            _ => return Ok(None),
        }))
    }

    pub fn write_xml(&self, elem: &mut BytesStart) {
        match self {
            StyleAttr::BackgroundColor(col) => {
                elem.push_attribute(("background-color", StyleAttr::color_to_str(*col).as_str()));
            }

            StyleAttr::BorderColor(col) => {
                elem.push_attribute(("border-color", StyleAttr::color_to_str(*col).as_str()));
            }

            StyleAttr::Display(disp) => {
                elem.push_attribute((
                    "display",
                    match disp {
                        Display::None => "none",
                        Display::Flex => "flex",
                        Display::Grid => "grid",
                    },
                ));
            }

            StyleAttr::Position(pos) => {
                elem.push_attribute((
                    "position",
                    match pos {
                        PositionType::Absolute => "absolute",
                        PositionType::Relative => "relative",
                    },
                ));
            }

            StyleAttr::Overflow(ov) => {
                elem.push_attribute((
                    "overflow",
                    match ov {
                        OverflowAxis::Clip => "clip",
                        OverflowAxis::Visible => "visible",
                    },
                ));
            }

            StyleAttr::OverflowX(ov) => {
                elem.push_attribute((
                    "overflow-x",
                    match ov {
                        OverflowAxis::Clip => "clip",
                        OverflowAxis::Visible => "visible",
                    },
                ));
            }

            StyleAttr::OverflowY(ov) => {
                elem.push_attribute((
                    "overflow-y",
                    match ov {
                        OverflowAxis::Clip => "clip",
                        OverflowAxis::Visible => "visible",
                    },
                ));
            }

            // StyleAttr::Direction(val) => {
            //     computed.direction = *val;
            // }
            StyleAttr::Left(val) => {
                elem.push_attribute(("left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Right(val) => {
                elem.push_attribute(("right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Top(val) => {
                elem.push_attribute(("top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Bottom(val) => {
                elem.push_attribute(("bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::Width(val) => {
                elem.push_attribute(("width", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Height(val) => {
                elem.push_attribute(("height", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MinWidth(val) => {
                elem.push_attribute(("min-width", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MinHeight(val) => {
                elem.push_attribute(("min-height", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MaxWidth(val) => {
                elem.push_attribute(("max-width", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MaxHeight(val) => {
                elem.push_attribute(("max-height", StyleAttr::val_to_str(*val).as_str()));
            }

            // StyleAttr::AlignItems(val) => {
            //     computed.align_items = *val;
            // }
            // StyleAttr::JustifyItems(val) => {
            //     computed.justify_items = *val;
            // }
            // StyleAttr::AlignSelf(val) => {
            //     computed.align_self = *val;
            // }
            // StyleAttr::JustifySelf(val) => {
            //     computed.justify_self = *val;
            // }
            StyleAttr::Margin(val) => {
                elem.push_attribute(("margin", StyleAttr::uirect_to_str(*val).as_str()));
            }
            StyleAttr::MarginLeft(val) => {
                elem.push_attribute(("margin-left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MarginRight(val) => {
                elem.push_attribute(("margin-right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MarginTop(val) => {
                elem.push_attribute(("margin-top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MarginBottom(val) => {
                elem.push_attribute(("margin-bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::Padding(val) => {
                elem.push_attribute(("padding", StyleAttr::uirect_to_str(*val).as_str()));
            }
            StyleAttr::PaddingLeft(val) => {
                elem.push_attribute(("padding-left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::PaddingRight(val) => {
                elem.push_attribute(("padding-right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::PaddingTop(val) => {
                elem.push_attribute(("padding-top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::PaddingBottom(val) => {
                elem.push_attribute(("padding-bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::Border(val) => {
                elem.push_attribute(("border", StyleAttr::uirect_to_str(*val).as_str()));
            }
            StyleAttr::BorderLeft(val) => {
                elem.push_attribute(("border-left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::BorderRight(val) => {
                elem.push_attribute(("border-right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::BorderTop(val) => {
                elem.push_attribute(("border-top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::BorderBottom(val) => {
                elem.push_attribute(("border-bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            // StyleAttr::FlexDirection(val) => {
            //     computed.flex_direction = *val;
            // }
            // StyleAttr::FlexWrap(val) => {
            //     computed.flex_wrap = *val;
            // }
            // StyleAttr::FlexGrow(val) => {
            //     computed.flex_grow = *val;
            // }
            // StyleAttr::FlexShrink(val) => {
            //     computed.flex_shrink = *val;
            // }
            // StyleAttr::FlexBasis(val) => {
            //     computed.flex_basis = *val;
            // }

            // StyleAttr::RowGap(val) => {
            //     computed.row_gap = *val;
            // }
            // StyleAttr::ColumnGap(val) => {
            //     computed.column_gap = *val;
            // }
            // StyleAttr::Gap(val) => {
            //     computed.row_gap = *val;
            //     computed.column_gap = *val;
            // }
            _ => {
                panic!("Unsupported tag")
            }
        }
    }

    /// Convert a CSS-style color into a Color. Supports #hex, rgba() and hsla().
    fn parse_color(str: &str) -> Result<Color, GuiseError> {
        lazy_static! {
            static ref RE_RGBA: Regex =
                Regex::new(r"^rgba\(([\d\.]+),\s*([\d\.]+),\s*([\d\.]+),\s*([\d\.]+)\)$").unwrap();
            static ref RE_HSLA: Regex =
                Regex::new(r"^hsla\(([\d\.]+),\s*([\d\.]+),\s*([\d\.]+),\s*([\d\.]+)\)$").unwrap();
        }

        let h = Color::hex(str);
        if h.is_ok() {
            return Ok(h.unwrap());
        }

        RE_RGBA
            .captures(str)
            .map(|cap| {
                Color::rgba(
                    f32::from_str(&cap[1]).unwrap(),
                    f32::from_str(&cap[2]).unwrap(),
                    f32::from_str(&cap[3]).unwrap(),
                    f32::from_str(&cap[4]).unwrap(),
                )
            })
            .or(RE_HSLA.captures(str).map(|cap| {
                Color::hsla(
                    f32::from_str(&cap[1]).unwrap(),
                    f32::from_str(&cap[2]).unwrap(),
                    f32::from_str(&cap[3]).unwrap(),
                    f32::from_str(&cap[4]).unwrap(),
                )
            }))
            .ok_or(GuiseError::InvalidAttributeValue(str.to_string()))
    }

    /// Convert a CSS-style length string into a `Val`.
    pub(crate) fn parse_val(str: &str) -> Result<Val, GuiseError> {
        if str == "auto" {
            return Ok(Val::Auto);
        }
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^([\-\d\.]+)(px|vw|vh|vmin|vmax|%)?$").unwrap();
        }
        RE.captures(str)
            .and_then(|cap| {
                let dist = f32::from_str(&cap[1]).unwrap();
                if cap.get(2).is_none() {
                    // Default to pixels if no unit
                    return Some(Val::Px(dist));
                }
                match &cap[2] {
                    "px" => Some(Val::Px(dist)),
                    "%" => Some(Val::Percent(dist)),
                    "vw" => Some(Val::Vw(dist)),
                    "vh" => Some(Val::Vh(dist)),
                    "vmin" => Some(Val::VMin(dist)),
                    "vmax" => Some(Val::VMax(dist)),
                    _ => {
                        panic!("Invalid unit");
                    }
                }
            })
            .ok_or(GuiseError::InvalidAttributeValue(str.to_string()))
    }

    /// Convert a CSS-style string representing a sequences of "lengths" into a `UiRect`.
    /// These go in CSS order: (top, right, bottom, left).
    /// CSS shortcut forms are supported.
    pub(crate) fn parse_uirect(str: &str) -> Result<UiRect, GuiseError> {
        let mut rect = UiRect::new(Val::Auto, Val::Auto, Val::Auto, Val::Auto);
        let mut sides = str.split_whitespace();

        // Top
        if let Some(top) = sides.next() {
            rect.top = StyleAttr::parse_val(top)?;

            // Right defaults to top if not specified
            rect.right = match sides.next() {
                Some(val) => StyleAttr::parse_val(val)?,
                None => rect.top,
            };

            // Bottom defaults to top if not specified
            rect.bottom = match sides.next() {
                Some(val) => StyleAttr::parse_val(val)?,
                None => rect.top,
            };

            // Left defaults to right if not specified
            rect.left = match sides.next() {
                Some(val) => StyleAttr::parse_val(val)?,
                None => rect.right,
            };

            // Should be no more values.
            if sides.next().is_none() {
                Ok(rect)
            } else {
                Err(GuiseError::InvalidAttributeValue(str.to_string()))
            }
        } else {
            Err(GuiseError::InvalidAttributeValue(str.to_string()))
        }
    }

    /// Convert a `Val` into a CSS-style string.
    fn val_to_str(val: Val) -> String {
        match val {
            Val::Auto => "auto".to_string(),
            Val::Px(px) => format!("{}px", px),
            Val::Percent(pct) => format!("{}%", pct),
            Val::Vw(v) => format!("{}vw", v),
            Val::Vh(v) => format!("{}vh", v),
            Val::VMin(v) => format!("{}vmin", v),
            Val::VMax(v) => format!("{}vmax", v),
        }
    }

    /// Convert a `UiRect` into a CSS-style string. The order of the values is (top, right, bottom,
    /// left).
    fn uirect_to_str(val: UiRect) -> String {
        format!(
            "{} {} {} {}",
            StyleAttr::val_to_str(val.top),
            StyleAttr::val_to_str(val.right),
            StyleAttr::val_to_str(val.bottom),
            StyleAttr::val_to_str(val.left)
        )
    }

    fn color_to_str(col: Color) -> String {
        match col {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => format!("rgba({}, {}, {}, {})", red, green, blue, alpha),

            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => format!("hsla({}, {}, {}, {})", hue, saturation, lightness, alpha),

            _ => {
                panic!("Unsupported color format")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_val() {
        assert_eq!(StyleAttr::parse_val("auto").unwrap(), Val::Auto);
        assert_eq!(StyleAttr::parse_val("1").unwrap(), Val::Px(1.));
        assert_eq!(StyleAttr::parse_val("1px").unwrap(), Val::Px(1.));
        assert_eq!(StyleAttr::parse_val("1vw").unwrap(), Val::Vw(1.));
        assert_eq!(StyleAttr::parse_val("1vh").unwrap(), Val::Vh(1.));
        assert_eq!(StyleAttr::parse_val("1.1px").unwrap(), Val::Px(1.1));

        assert!(StyleAttr::parse_val("1.1bad").is_err());
        assert!(StyleAttr::parse_val("bad").is_err());
        assert!(StyleAttr::parse_val("1.1.1bad").is_err());
    }

    #[test]
    fn test_parse_uirect() {
        assert_eq!(
            StyleAttr::parse_uirect("1px 2px 3px 4px").unwrap(),
            UiRect::new(Val::Px(4.), Val::Px(2.), Val::Px(1.), Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse_uirect("1px 2px 3px").unwrap(),
            UiRect::new(Val::Px(2.), Val::Px(2.), Val::Px(1.), Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse_uirect("1px 2px").unwrap(),
            UiRect::new(Val::Px(2.), Val::Px(2.), Val::Px(1.), Val::Px(1.))
        );
        assert_eq!(
            StyleAttr::parse_uirect("1px").unwrap(),
            UiRect::new(Val::Px(1.), Val::Px(1.), Val::Px(1.), Val::Px(1.))
        );

        assert!(StyleAttr::parse_uirect("1.1bad").is_err());
    }
}
