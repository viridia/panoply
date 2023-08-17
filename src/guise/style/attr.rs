use bevy::{prelude::Color, ui::*};
use lazy_static::lazy_static;
use quick_xml::events::BytesStart;
use regex::Regex;
use std::str::FromStr;

use crate::guise::GuiseError;

use super::ComputedStyle;

/** Set of style attributes that can be applied to construct a style. */
#[derive(Debug, Clone, PartialEq)]
pub enum StyleAttr {
    BackgroundColor(Option<Color>),
    BorderColor(Option<Color>),
    ZIndex(i32),

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
    AlignContent(bevy::ui::AlignContent),
    JustifyContent(bevy::ui::JustifyContent),

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
    GridAutoFlow(bevy::ui::GridAutoFlow),
    // pub grid_auto_flow: StyleProp<GridAutoFlow>,
    // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
    // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
    // pub grid_auto_rows: Option<Vec<GridTrack>>,
    // pub grid_auto_columns: Option<Vec<GridTrack>>,
    GridRow(bevy::ui::GridPlacement),
    GridRowStart(i16),
    GridRowSpan(u16),
    GridRowEnd(i16),
    GridColumn(bevy::ui::GridPlacement),
    GridColumnStart(i16),
    GridColumnSpan(u16),
    GridColumnEnd(i16),
    // pub grid_row: GridPlacement,
    // pub grid_column: GridPlacement,
}

impl StyleAttr {
    /// Apply this style attribute to a computed style.
    pub fn apply(&self, computed: &mut ComputedStyle) {
        match self {
            StyleAttr::BackgroundColor(val) => {
                computed.background_color = *val;
            }
            StyleAttr::BorderColor(val) => {
                computed.border_color = *val;
            }
            StyleAttr::ZIndex(val) => {
                computed.z_index = Some(*val);
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
            StyleAttr::AlignContent(val) => {
                computed.style.align_content = *val;
            }
            StyleAttr::JustifyContent(val) => {
                computed.style.justify_content = *val;
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

            StyleAttr::GridAutoFlow(val) => {
                computed.style.grid_auto_flow = *val;
            }
            StyleAttr::GridRow(val) => {
                computed.style.grid_row = *val;
            }
            StyleAttr::GridRowStart(val) => {
                computed.style.grid_row.set_start(*val);
            }
            StyleAttr::GridRowSpan(val) => {
                computed.style.grid_row.set_span(*val);
            }
            StyleAttr::GridRowEnd(val) => {
                computed.style.grid_row.set_end(*val);
            }

            StyleAttr::GridColumn(val) => {
                computed.style.grid_column = *val;
            }
            StyleAttr::GridColumnStart(val) => {
                computed.style.grid_column.set_start(*val);
            }
            StyleAttr::GridColumnSpan(val) => {
                computed.style.grid_column.set_span(*val);
            }
            StyleAttr::GridColumnEnd(val) => {
                computed.style.grid_column.set_end(*val);
            }
        }
    }

    /// Parse a `StyleAttr` from an XML attribute name/value pair.
    pub fn parse<'a>(name: &'a [u8], value: &str) -> Result<Option<Self>, GuiseError> {
        Ok(Some(match name {
            b"background-color" => StyleAttr::BackgroundColor(if value == "transparent" {
                None
            } else {
                Some(StyleAttr::parse_color(value)?)
            }),

            b"border-color" => StyleAttr::BorderColor(if value == "transparent" {
                None
            } else {
                Some(StyleAttr::parse_color(value)?)
            }),

            b"z-index" => StyleAttr::ZIndex(StyleAttr::parse_i32(value)?),

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

            b"direction" => StyleAttr::Direction(match value {
                "inherit" => bevy::ui::Direction::Inherit,
                "ltr" => bevy::ui::Direction::LeftToRight,
                "rtl" => bevy::ui::Direction::RightToLeft,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

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
            b"align-items" => StyleAttr::AlignItems(match value {
                "default" => AlignItems::Default,
                "start" => AlignItems::Start,
                "end" => AlignItems::End,
                "flex-start" => AlignItems::FlexStart,
                "flex-end" => AlignItems::FlexEnd,
                "center" => AlignItems::Center,
                "baseline" => AlignItems::Baseline,
                "stretch" => AlignItems::Stretch,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"justify-items" => StyleAttr::JustifyItems(match value {
                "default" => JustifyItems::Default,
                "start" => JustifyItems::Start,
                "end" => JustifyItems::End,
                "center" => JustifyItems::Center,
                "baseline" => JustifyItems::Baseline,
                "stretch" => JustifyItems::Stretch,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"align-self" => StyleAttr::AlignSelf(match value {
                "auto" => AlignSelf::Auto,
                "start" => AlignSelf::Start,
                "end" => AlignSelf::End,
                "flex-start" => AlignSelf::FlexStart,
                "flex-end" => AlignSelf::FlexEnd,
                "center" => AlignSelf::Center,
                "baseline" => AlignSelf::Baseline,
                "stretch" => AlignSelf::Stretch,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"justify-self" => StyleAttr::JustifySelf(match value {
                "auto" => JustifySelf::Auto,
                "start" => JustifySelf::Start,
                "end" => JustifySelf::End,
                "center" => JustifySelf::Center,
                "baseline" => JustifySelf::Baseline,
                "stretch" => JustifySelf::Stretch,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"align-content" => StyleAttr::AlignContent(match value {
                "default" => AlignContent::Default,
                "start" => AlignContent::Start,
                "end" => AlignContent::End,
                "flex-start" => AlignContent::FlexStart,
                "flex-end" => AlignContent::FlexEnd,
                "center" => AlignContent::Center,
                "stretch" => AlignContent::Stretch,
                "space-between" => AlignContent::SpaceBetween,
                "space-around" => AlignContent::SpaceAround,
                "space-evenly" => AlignContent::SpaceEvenly,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"justify-content" => StyleAttr::JustifyContent(match value {
                "default" => JustifyContent::Default,
                "start" => JustifyContent::Start,
                "end" => JustifyContent::End,
                "flex-start" => JustifyContent::FlexStart,
                "flex-end" => JustifyContent::FlexEnd,
                "center" => JustifyContent::Center,
                "space-between" => JustifyContent::SpaceBetween,
                "space-around" => JustifyContent::SpaceAround,
                "space-evenly" => JustifyContent::SpaceEvenly,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

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

            b"flex-wrap" => StyleAttr::FlexWrap(match value {
                "nowrap" => FlexWrap::NoWrap,
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            // TODO: Allow shortcut forms for flex.
            b"flex" => StyleAttr::FlexGrow(StyleAttr::parse_f32(value)?),
            b"flex-grow" => StyleAttr::FlexGrow(StyleAttr::parse_f32(value)?),
            b"flex-shrink" => StyleAttr::FlexShrink(StyleAttr::parse_f32(value)?),
            b"flex-basis" => StyleAttr::FlexBasis(StyleAttr::parse_val(value)?),

            b"row-gap" => StyleAttr::RowGap(StyleAttr::parse_val(value)?),
            b"column-gap" => StyleAttr::ColumnGap(StyleAttr::parse_val(value)?),
            b"gap" => StyleAttr::Gap(StyleAttr::parse_val(value)?),

            b"grid-auto-flow" => StyleAttr::GridAutoFlow(match value {
                "row" => GridAutoFlow::Row,
                "column" => GridAutoFlow::Column,
                "row-dense" => GridAutoFlow::RowDense,
                "column-dense" => GridAutoFlow::ColumnDense,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),
            //     // TODO:
            //     // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
            //     // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
            //     // pub grid_auto_rows: Option<Vec<GridTrack>>,
            //     // pub grid_auto_columns: Option<Vec<GridTrack>>,
            b"grid-row" => StyleAttr::GridRow(StyleAttr::parse_grid_placement(value)?),
            b"grid-row-start" => StyleAttr::GridRowStart(StyleAttr::parse_i16(value)?),
            b"grid-row-span" => StyleAttr::GridRowSpan(StyleAttr::parse_u16(value)?),
            b"grid-row-end" => StyleAttr::GridRowEnd(StyleAttr::parse_i16(value)?),
            b"grid-column" => StyleAttr::GridColumn(StyleAttr::parse_grid_placement(value)?),
            b"grid-column-start" => StyleAttr::GridColumnStart(StyleAttr::parse_i16(value)?),
            b"grid-column-span" => StyleAttr::GridColumnSpan(StyleAttr::parse_u16(value)?),
            b"grid-column-end" => StyleAttr::GridColumnEnd(StyleAttr::parse_i16(value)?),
            _ => return Ok(None),
        }))
    }

    pub fn write_xml(&self, elem: &mut BytesStart) {
        match self {
            StyleAttr::BackgroundColor(Some(col)) => {
                elem.push_attribute(("background-color", StyleAttr::color_to_str(*col).as_str()));
            }
            StyleAttr::BackgroundColor(None) => {
                elem.push_attribute(("background-color", "transparent"));
            }

            StyleAttr::BorderColor(Some(col)) => {
                elem.push_attribute(("border-color", StyleAttr::color_to_str(*col).as_str()));
            }
            StyleAttr::BorderColor(None) => {
                elem.push_attribute(("border-color", "transparent"));
            }

            StyleAttr::ZIndex(val) => {
                elem.push_attribute(("z-index", val.to_string().as_str()));
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

            StyleAttr::Direction(dir) => {
                elem.push_attribute((
                    "direction",
                    match dir {
                        bevy::ui::Direction::Inherit => "inherit",
                        bevy::ui::Direction::LeftToRight => "ltr",
                        bevy::ui::Direction::RightToLeft => "rtl",
                    },
                ));
            }

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

            StyleAttr::AlignItems(align) => {
                elem.push_attribute((
                    "align-items",
                    match align {
                        AlignItems::Default => "default",
                        AlignItems::Start => "start",
                        AlignItems::End => "end",
                        AlignItems::FlexStart => "flex-start",
                        AlignItems::FlexEnd => "flex-end",
                        AlignItems::Center => "center",
                        AlignItems::Baseline => "baseline",
                        AlignItems::Stretch => "stretch",
                    },
                ));
            }

            StyleAttr::JustifyItems(align) => {
                elem.push_attribute((
                    "justify-items",
                    match align {
                        JustifyItems::Default => "default",
                        JustifyItems::Start => "start",
                        JustifyItems::End => "end",
                        JustifyItems::Center => "center",
                        JustifyItems::Baseline => "baseline",
                        JustifyItems::Stretch => "stretch",
                    },
                ));
            }

            StyleAttr::AlignSelf(align) => {
                elem.push_attribute((
                    "align-self",
                    match align {
                        AlignSelf::Auto => "auto",
                        AlignSelf::Start => "start",
                        AlignSelf::End => "end",
                        AlignSelf::FlexStart => "flex-start",
                        AlignSelf::FlexEnd => "flex-end",
                        AlignSelf::Center => "center",
                        AlignSelf::Baseline => "baseline",
                        AlignSelf::Stretch => "stretch",
                    },
                ));
            }

            StyleAttr::JustifySelf(align) => {
                elem.push_attribute((
                    "justify-self",
                    match align {
                        JustifySelf::Auto => "auto",
                        JustifySelf::Start => "start",
                        JustifySelf::End => "end",
                        JustifySelf::Center => "center",
                        JustifySelf::Baseline => "baseline",
                        JustifySelf::Stretch => "stretch",
                    },
                ));
            }

            StyleAttr::AlignContent(align) => {
                elem.push_attribute((
                    "align-content",
                    match align {
                        AlignContent::Default => "default",
                        AlignContent::Start => "start",
                        AlignContent::End => "end",
                        AlignContent::FlexStart => "flex-start",
                        AlignContent::FlexEnd => "flex-end",
                        AlignContent::Center => "center",
                        AlignContent::Stretch => "stretch",
                        AlignContent::SpaceBetween => "space-between",
                        AlignContent::SpaceAround => "space-around",
                        AlignContent::SpaceEvenly => "space-evenly",
                    },
                ));
            }

            StyleAttr::JustifyContent(align) => {
                elem.push_attribute((
                    "justify-content",
                    match align {
                        JustifyContent::Default => "default",
                        JustifyContent::Start => "start",
                        JustifyContent::End => "end",
                        JustifyContent::FlexStart => "flex-start",
                        JustifyContent::FlexEnd => "flex-end",
                        JustifyContent::Center => "center",
                        JustifyContent::SpaceBetween => "space-between",
                        JustifyContent::SpaceAround => "space-around",
                        JustifyContent::SpaceEvenly => "space-evenly",
                    },
                ));
            }

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

            StyleAttr::FlexDirection(dir) => {
                elem.push_attribute((
                    "flex-direction",
                    match dir {
                        bevy::ui::FlexDirection::Row => "row",
                        bevy::ui::FlexDirection::Column => "column",
                        bevy::ui::FlexDirection::RowReverse => "row-reverse",
                        bevy::ui::FlexDirection::ColumnReverse => "column-reverse",
                    },
                ));
            }

            StyleAttr::FlexWrap(dir) => {
                elem.push_attribute((
                    "flex-wrap",
                    match dir {
                        bevy::ui::FlexWrap::NoWrap => "nowrap",
                        bevy::ui::FlexWrap::Wrap => "wrap",
                        bevy::ui::FlexWrap::WrapReverse => "wrap-reverse",
                    },
                ));
            }

            StyleAttr::FlexGrow(val) => {
                elem.push_attribute(("flex-grow", f32::to_string(val).as_str()));
            }
            StyleAttr::FlexShrink(val) => {
                elem.push_attribute(("flex-shrink", f32::to_string(val).as_str()));
            }
            StyleAttr::FlexBasis(val) => {
                elem.push_attribute(("flex-basis", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::RowGap(val) => {
                elem.push_attribute(("row-gap", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::ColumnGap(val) => {
                elem.push_attribute(("column-gap", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Gap(val) => {
                elem.push_attribute(("gap", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::GridAutoFlow(val) => {
                elem.push_attribute((
                    "grid-auto-flow",
                    match val {
                        GridAutoFlow::Row => "row",
                        GridAutoFlow::Column => "column",
                        GridAutoFlow::RowDense => "row-dense",
                        GridAutoFlow::ColumnDense => "column-dense",
                    },
                ));
            }

            StyleAttr::GridRow(_) => {
                panic!("Unsupported, can't write GridPlacement");
            }
            StyleAttr::GridRowStart(val) => {
                elem.push_attribute(("grid-row-start", i16::to_string(val).as_str()));
            }
            StyleAttr::GridRowSpan(val) => {
                elem.push_attribute(("grid-row-span", u16::to_string(val).as_str()));
            }
            StyleAttr::GridRowEnd(val) => {
                elem.push_attribute(("grid-row-end", i16::to_string(val).as_str()));
            }

            StyleAttr::GridColumn(_) => {
                panic!("Unsupported, can't write GridPlacement");
            }
            StyleAttr::GridColumnStart(val) => {
                elem.push_attribute(("grid-column-start", i16::to_string(val).as_str()));
            }
            StyleAttr::GridColumnSpan(val) => {
                elem.push_attribute(("grid-column-span", u16::to_string(val).as_str()));
            }
            StyleAttr::GridColumnEnd(val) => {
                elem.push_attribute(("grid-column-end", i16::to_string(val).as_str()));
            } // _ => {
              //     panic!("Unsupported tag")
              // }
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

    /// Convert a CSS-style color into a Color. Supports #hex, rgba() and hsla().
    fn parse_grid_placement(str: &str) -> Result<GridPlacement, GuiseError> {
        lazy_static! {
            static ref RE_GRID_1: Regex = Regex::new(r"^([\d\.]+)\s*/\s*([\d\.]+)$").unwrap();
            static ref RE_GRID_2: Regex =
                Regex::new(r"^([\d\.]+)\s*/\s*span\s*([\d\.]+)$").unwrap();
        }

        RE_GRID_1
            .captures(str)
            .map(|cap| {
                GridPlacement::default()
                    .set_start(i16::from_str(&cap[1]).unwrap())
                    .set_end(i16::from_str(&cap[2]).unwrap())
            })
            .or(RE_GRID_2.captures(str).map(|cap| {
                GridPlacement::default()
                    .set_start(i16::from_str(&cap[1]).unwrap())
                    .set_span(u16::from_str(&cap[2]).unwrap())
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

    /// Parse a scalar float.
    fn parse_f32(str: &str) -> Result<f32, GuiseError> {
        f32::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
    }

    /// Parse a scalar i32.
    fn parse_i32(str: &str) -> Result<i32, GuiseError> {
        i32::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
    }

    /// Parse a scalar i32.
    fn parse_i16(str: &str) -> Result<i16, GuiseError> {
        i16::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
    }

    /// Parse a scalar i32.
    fn parse_u16(str: &str) -> Result<u16, GuiseError> {
        u16::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
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

    #[test]
    fn test_parse_attrs() {
        assert_eq!(
            StyleAttr::parse(b"background-color", "#123")
                .unwrap()
                .unwrap(),
            StyleAttr::BackgroundColor(Some(Color::hex("#123").unwrap()))
        );

        assert_eq!(
            StyleAttr::parse(b"background-color", "transparent")
                .unwrap()
                .unwrap(),
            StyleAttr::BackgroundColor(None)
        );

        assert_eq!(
            StyleAttr::parse(b"border-color", "#123").unwrap().unwrap(),
            StyleAttr::BorderColor(Some(Color::hex("#123").unwrap()))
        );

        assert_eq!(
            StyleAttr::parse(b"border-color", "transparent")
                .unwrap()
                .unwrap(),
            StyleAttr::BorderColor(None)
        );

        assert_eq!(
            StyleAttr::parse(b"z-index", "33").unwrap().unwrap(),
            StyleAttr::ZIndex(33)
        );

        assert_eq!(
            StyleAttr::parse(b"display", "none").unwrap().unwrap(),
            StyleAttr::Display(bevy::ui::Display::None)
        );
        assert_eq!(
            StyleAttr::parse(b"display", "flex").unwrap().unwrap(),
            StyleAttr::Display(bevy::ui::Display::Flex)
        );
        assert_eq!(
            StyleAttr::parse(b"display", "grid").unwrap().unwrap(),
            StyleAttr::Display(bevy::ui::Display::Grid)
        );

        assert_eq!(
            StyleAttr::parse(b"position", "absolute").unwrap().unwrap(),
            StyleAttr::Position(bevy::ui::PositionType::Absolute)
        );
        assert_eq!(
            StyleAttr::parse(b"position", "relative").unwrap().unwrap(),
            StyleAttr::Position(bevy::ui::PositionType::Relative)
        );

        assert_eq!(
            StyleAttr::parse(b"overflow", "clip").unwrap().unwrap(),
            StyleAttr::Overflow(bevy::ui::OverflowAxis::Clip)
        );
        assert_eq!(
            StyleAttr::parse(b"overflow", "visible").unwrap().unwrap(),
            StyleAttr::Overflow(bevy::ui::OverflowAxis::Visible)
        );
        assert_eq!(
            StyleAttr::parse(b"overflow-x", "clip").unwrap().unwrap(),
            StyleAttr::OverflowX(bevy::ui::OverflowAxis::Clip)
        );
        assert_eq!(
            StyleAttr::parse(b"overflow-x", "visible").unwrap().unwrap(),
            StyleAttr::OverflowX(bevy::ui::OverflowAxis::Visible)
        );
        assert_eq!(
            StyleAttr::parse(b"overflow-y", "clip").unwrap().unwrap(),
            StyleAttr::OverflowY(bevy::ui::OverflowAxis::Clip)
        );
        assert_eq!(
            StyleAttr::parse(b"overflow-y", "visible").unwrap().unwrap(),
            StyleAttr::OverflowY(bevy::ui::OverflowAxis::Visible)
        );

        assert_eq!(
            StyleAttr::parse(b"direction", "inherit").unwrap().unwrap(),
            StyleAttr::Direction(bevy::ui::Direction::Inherit)
        );
        assert_eq!(
            StyleAttr::parse(b"direction", "ltr").unwrap().unwrap(),
            StyleAttr::Direction(bevy::ui::Direction::LeftToRight)
        );
        assert_eq!(
            StyleAttr::parse(b"direction", "rtl").unwrap().unwrap(),
            StyleAttr::Direction(bevy::ui::Direction::RightToLeft)
        );

        assert_eq!(
            StyleAttr::parse(b"left", "3").unwrap().unwrap(),
            StyleAttr::Left(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"right", "3").unwrap().unwrap(),
            StyleAttr::Right(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"top", "3").unwrap().unwrap(),
            StyleAttr::Top(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"bottom", "3").unwrap().unwrap(),
            StyleAttr::Bottom(bevy::ui::Val::Px(3.))
        );

        assert_eq!(
            StyleAttr::parse(b"width", "3").unwrap().unwrap(),
            StyleAttr::Width(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"height", "3").unwrap().unwrap(),
            StyleAttr::Height(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"min-width", "3").unwrap().unwrap(),
            StyleAttr::MinWidth(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"min-height", "3").unwrap().unwrap(),
            StyleAttr::MinHeight(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"max-width", "3").unwrap().unwrap(),
            StyleAttr::MaxWidth(bevy::ui::Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse(b"max-height", "3").unwrap().unwrap(),
            StyleAttr::MaxHeight(bevy::ui::Val::Px(3.))
        );

        //         //     // pub aspect_ratio: StyleProp<f32>,
        //         b"align-items" => StyleAttr::AlignItems(match value {
        //             "default" => AlignItems::Default,
        //             "start" => AlignItems::Start,
        //             "end" => AlignItems::End,
        //             "flex-start" => AlignItems::FlexStart,
        //             "flex-end" => AlignItems::FlexEnd,
        //             "center" => AlignItems::Center,
        //             "baseline" => AlignItems::Baseline,
        //             "stretch" => AlignItems::Stretch,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         b"justify-items" => StyleAttr::JustifyItems(match value {
        //             "default" => JustifyItems::Default,
        //             "start" => JustifyItems::Start,
        //             "end" => JustifyItems::End,
        //             "center" => JustifyItems::Center,
        //             "baseline" => JustifyItems::Baseline,
        //             "stretch" => JustifyItems::Stretch,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         b"align-self" => StyleAttr::AlignSelf(match value {
        //             "auto" => AlignSelf::Auto,
        //             "start" => AlignSelf::Start,
        //             "end" => AlignSelf::End,
        //             "flex-start" => AlignSelf::FlexStart,
        //             "flex-end" => AlignSelf::FlexEnd,
        //             "center" => AlignSelf::Center,
        //             "baseline" => AlignSelf::Baseline,
        //             "stretch" => AlignSelf::Stretch,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         b"justify-self" => StyleAttr::JustifySelf(match value {
        //             "auto" => JustifySelf::Auto,
        //             "start" => JustifySelf::Start,
        //             "end" => JustifySelf::End,
        //             "center" => JustifySelf::Center,
        //             "baseline" => JustifySelf::Baseline,
        //             "stretch" => JustifySelf::Stretch,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         b"align-content" => StyleAttr::AlignContent(match value {
        //             "default" => AlignContent::Default,
        //             "start" => AlignContent::Start,
        //             "end" => AlignContent::End,
        //             "flex-start" => AlignContent::FlexStart,
        //             "flex-end" => AlignContent::FlexEnd,
        //             "center" => AlignContent::Center,
        //             "stretch" => AlignContent::Stretch,
        //             "space-between" => AlignContent::SpaceBetween,
        //             "space-around" => AlignContent::SpaceAround,
        //             "space-evenly" => AlignContent::SpaceEvenly,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         b"justify-content" => StyleAttr::JustifyContent(match value {
        //             "default" => JustifyContent::Default,
        //             "start" => JustifyContent::Start,
        //             "end" => JustifyContent::End,
        //             "flex-start" => JustifyContent::FlexStart,
        //             "flex-end" => JustifyContent::FlexEnd,
        //             "center" => JustifyContent::Center,
        //             "space-between" => JustifyContent::SpaceBetween,
        //             "space-around" => JustifyContent::SpaceAround,
        //             "space-evenly" => JustifyContent::SpaceEvenly,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         b"margin" => StyleAttr::Margin(StyleAttr::parse_uirect(value)?),
        //         b"margin-left" => StyleAttr::MarginLeft(StyleAttr::parse_val(value)?),
        //         b"margin-right" => StyleAttr::MarginRight(StyleAttr::parse_val(value)?),
        //         b"margin-top" => StyleAttr::MarginTop(StyleAttr::parse_val(value)?),
        //         b"margin-bottom" => StyleAttr::MarginBottom(StyleAttr::parse_val(value)?),

        //         b"padding" => StyleAttr::Padding(StyleAttr::parse_uirect(value)?),
        //         b"padding-left" => StyleAttr::PaddingLeft(StyleAttr::parse_val(value)?),
        //         b"padding-right" => StyleAttr::PaddingRight(StyleAttr::parse_val(value)?),
        //         b"padding-top" => StyleAttr::PaddingTop(StyleAttr::parse_val(value)?),
        //         b"padding-bottom" => StyleAttr::PaddingBottom(StyleAttr::parse_val(value)?),

        //         b"border" => StyleAttr::Border(StyleAttr::parse_uirect(value)?),
        //         b"border-left" => StyleAttr::BorderLeft(StyleAttr::parse_val(value)?),
        //         b"border-right" => StyleAttr::BorderRight(StyleAttr::parse_val(value)?),
        //         b"border-top" => StyleAttr::BorderTop(StyleAttr::parse_val(value)?),
        //         b"border-bottom" => StyleAttr::BorderBottom(StyleAttr::parse_val(value)?),

        //         b"flex-direction" => StyleAttr::FlexDirection(match value {
        //             "row" => FlexDirection::Row,
        //             "column" => FlexDirection::Column,
        //             "row-reverse" => FlexDirection::RowReverse,
        //             "column-reverse" => FlexDirection::ColumnReverse,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),

        //         //     FlexWrap,
        //         // TODO: Allow shortcut forms for flex.
        //         b"flex" => StyleAttr::FlexGrow(StyleAttr::parse_f32(value)?),
        //         b"flex-grow" => StyleAttr::FlexGrow(StyleAttr::parse_f32(value)?),
        //         b"flex-shrink" => StyleAttr::FlexShrink(StyleAttr::parse_f32(value)?),
        //         b"flex-basis" => StyleAttr::FlexBasis(StyleAttr::parse_val(value)?),

        //         b"row-gap" => StyleAttr::RowGap(StyleAttr::parse_val(value)?),
        //         b"column-gap" => StyleAttr::ColumnGap(StyleAttr::parse_val(value)?),
        //         b"gap" => StyleAttr::Gap(StyleAttr::parse_val(value)?),

        //         b"grid-auto-flow" => StyleAttr::GridAutoFlow(match value {
        //             "row" => GridAutoFlow::Row,
        //             "column" => GridAutoFlow::Column,
        //             "row-dense" => GridAutoFlow::RowDense,
        //             "column-dense" => GridAutoFlow::ColumnDense,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),
        //         //     // TODO:
        //         //     // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
        //         //     // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
        //         //     // pub grid_auto_rows: Option<Vec<GridTrack>>,
        //         //     // pub grid_auto_columns: Option<Vec<GridTrack>>,
        //         b"grid-row" => StyleAttr::GridRow(StyleAttr::parse_grid_placement(value)?),
        //         b"grid-row-start" => StyleAttr::GridRowStart(StyleAttr::parse_i16(value)?),
        //         b"grid-row-span" => StyleAttr::GridRowSpan(StyleAttr::parse_u16(value)?),
        //         b"grid-row-end" => StyleAttr::GridRowEnd(StyleAttr::parse_i16(value)?),
        //         b"grid-column" => StyleAttr::GridColumn(StyleAttr::parse_grid_placement(value)?),
        //         b"grid-column-start" => StyleAttr::GridColumnStart(StyleAttr::parse_i16(value)?),
        //         b"grid-column-span" => StyleAttr::GridColumnSpan(StyleAttr::parse_u16(value)?),
        //         b"grid-column-end" => StyleAttr::GridColumnEnd(StyleAttr::parse_i16(value)?),
        //         _ => return Ok(None),
        //     }))
        // }
    }
}
