use std::{fmt, marker::PhantomData};

use bevy::{
    prelude::Color,
    ui::{self, Val},
};
use serde::{de::Visitor, Deserialize, Serialize};

use super::{color::ColorValue, expr::Expr};

/// Enum representing possible values of a style attribute
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue<T> {
    // TODO: Possible other choices, assuming we can figure out how to serialize them.
    // Initial - use default value for type
    // Inherit - use value from parent computed
    //
    /// The value has been set to a constant
    Constant(T),

    /// The value has been set to an evaluable expression.
    Expr(Expr),
}

trait SerializableValue<T> {
    fn from_expr<E: serde::de::Error>(e: Expr) -> Result<StyleValue<T>, E>;

    fn visit_i32<E: serde::de::Error>(v: i32) -> Result<T, E> {
        Err(E::invalid_type(
            serde::de::Unexpected::Signed(v as i64),
            &"Invalid type i32",
        ))
    }

    fn visit_u32<E: serde::de::Error>(v: u32) -> Result<T, E> {
        Err(E::invalid_type(
            serde::de::Unexpected::Unsigned(v as u64),
            &"Invalid type u32",
        ))
    }

    fn visit_f32<E: serde::de::Error>(v: f32) -> Result<T, E> {
        Err(E::invalid_type(
            serde::de::Unexpected::Float(v as f64),
            &"Invalid type f32",
        ))
    }

    fn serialize_value<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
}

impl<T> StyleValue<T> {}

impl SerializableValue<i32> for StyleValue<i32> {
    fn from_expr<E: serde::de::Error>(e: Expr) -> Result<Self, E> {
        match e {
            Expr::Number(n) => Ok(StyleValue::Constant(n as i32)),
            Expr::Var(_) => Ok(StyleValue::Expr(e)),
            _ => Err(E::invalid_type(
                serde::de::Unexpected::Other("non-number"),
                &"i32",
            )),
        }
    }

    fn visit_i32<E: serde::de::Error>(v: i32) -> Result<i32, E> {
        Ok(v)
    }

    fn visit_u32<E: serde::de::Error>(v: u32) -> Result<i32, E> {
        Ok(v as i32)
    }

    fn serialize_value<S>(value: &i32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(*value)
    }
}

impl SerializableValue<f32> for StyleValue<f32> {
    fn from_expr<E: serde::de::Error>(e: Expr) -> Result<Self, E> {
        match e {
            Expr::Number(n) => Ok(StyleValue::Constant(n)),
            _ => Err(E::invalid_type(
                serde::de::Unexpected::Other("expression"),
                &"Invalid type",
            )),
        }
    }

    fn visit_i32<E: serde::de::Error>(v: i32) -> Result<f32, E> {
        Ok(v as f32)
    }

    fn visit_u32<E: serde::de::Error>(v: u32) -> Result<f32, E> {
        Ok(v as f32)
    }

    fn visit_f32<E: serde::de::Error>(v: f32) -> Result<f32, E> {
        Ok(v)
    }

    fn serialize_value<S>(value: &f32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f32(*value)
    }
}

impl SerializableValue<ColorValue> for StyleValue<ColorValue> {
    fn from_expr<E: serde::de::Error>(e: Expr) -> Result<Self, E> {
        match e {
            Expr::Ident(ref str) if str == "transparent" => {
                Ok(StyleValue::Constant(ColorValue::Transparent))
            }
            Expr::Color(c) => Ok(StyleValue::Constant(c)),
            _ => Err(E::invalid_type(
                serde::de::Unexpected::Other("expression"),
                &"Invalid type",
            )),
        }
    }

    fn serialize_value<S>(value: &ColorValue, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            ColorValue::Transparent => serializer.serialize_str("transparent"),
            ColorValue::Color(color) => match color {
                Color::Rgba {
                    red,
                    green,
                    blue,
                    alpha,
                } => serializer.serialize_str(
                    format!(
                        "rgba({}, {}, {}, {})",
                        red * 255.0,
                        green * 255.0,
                        blue * 255.0,
                        alpha
                    )
                    .as_str(),
                ),

                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha,
                } => serializer.serialize_str(
                    format!("hsla({}, {}, {}, {})", hue, saturation, lightness, alpha).as_str(),
                ),

                _ => {
                    panic!("Unsupported color format")
                }
            },
        }
    }
}

impl SerializableValue<Val> for StyleValue<Val> {
    fn from_expr<E: serde::de::Error>(e: Expr) -> Result<Self, E> {
        match e {
            Expr::Number(n) => Ok(Self::Constant(Val::Px(n))),
            Expr::Length(l) => Ok(Self::Constant(l)),
            _ => Err(E::invalid_type(
                serde::de::Unexpected::Other("expression"),
                &"Invalid type",
            )),
        }
    }

    fn visit_i32<E: serde::de::Error>(v: i32) -> Result<Val, E> {
        Ok(Val::Px(v as f32))
    }

    fn visit_u32<E: serde::de::Error>(v: u32) -> Result<Val, E> {
        Ok(Val::Px(v as f32))
    }

    fn visit_f32<E: serde::de::Error>(v: f32) -> Result<Val, E> {
        Ok(Val::Px(v))
    }

    fn serialize_value<S>(value: &Val, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Val::Auto => serializer.serialize_str("auto"),
            // Default unit is pixels
            Val::Px(v) => serializer.serialize_f32(*v),
            Val::Percent(v) => serializer.serialize_str(&format!("{}%", *v)),
            Val::Vw(v) => serializer.serialize_str(&format!("{}vw", *v)),
            Val::Vh(v) => serializer.serialize_str(&format!("{}vh", *v)),
            Val::VMin(v) => serializer.serialize_str(&format!("{}vmin", *v)),
            Val::VMax(v) => serializer.serialize_str(&format!("{}vmax", *v)),
        }
    }
}

impl<T> Serialize for StyleValue<T>
where
    StyleValue<T>: SerializableValue<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Constant(v) => StyleValue::<T>::serialize_value(v, serializer),
            Self::Expr(e) => e.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for StyleValue<T>
where
    StyleValue<T>: SerializableValue<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StyleValueVisitor {
            result_type: PhantomData::<T>,
        })
    }
}

struct StyleValueVisitor<T> {
    result_type: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for StyleValueVisitor<T>
where
    StyleValue<T>: SerializableValue<T>,
{
    type Value = StyleValue<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("style value")
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(StyleValue::Constant(StyleValue::<T>::visit_i32(v)?))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(StyleValue::Constant(StyleValue::<T>::visit_u32(v)?))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(StyleValue::Constant(StyleValue::<T>::visit_u32(v as u32)?))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(StyleValue::Constant(StyleValue::<T>::visit_f32(v)?))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(StyleValue::Constant(StyleValue::<T>::visit_f32(v as f32)?))
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s.parse::<Expr>() {
            Ok(expr) => StyleValue::<T>::from_expr(expr),
            Err(_) => Err(E::invalid_type(
                serde::de::Unexpected::Other("expr"),
                &"Invalid type",
            )),
        }
    }
}

impl StyleValue<ColorValue> {
    /// True if the color is 'transparent'.
    pub fn is_transparent(&self) -> bool {
        match self {
            Self::Constant(color) => color.is_transparent(),
            Self::Expr(_expr) => {
                todo!("Evaluate expression")
            }
        }
    }

    /// Extract the color value
    pub fn to_color_value(&self) -> ColorValue {
        match self {
            Self::Constant(color) => *color,
            Self::Expr(_expr) => {
                todo!("Evaluate expression")
            }
        }
    }

    /// Extract the actual color
    pub fn to_color(&self) -> Color {
        match self {
            Self::Constant(color) => color.color(),
            Self::Expr(_expr) => {
                todo!("Evaluate expression")
            }
        }
    }
}

impl StyleValue<i32> {
    /// Extract the color value
    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Constant(v) => *v,
            Self::Expr(_expr) => {
                todo!("Evaluate expression")
            }
        }
    }
}

impl StyleValue<f32> {
    /// Extract the color value
    pub fn to_f32(&self) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Expr(_expr) => {
                todo!("Evaluate expression")
            }
        }
    }
}

impl StyleValue<Val> {
    /// Extract the color value
    pub fn to_val(&self) -> ui::Val {
        match self {
            Self::Constant(v) => *v,
            Self::Expr(_expr) => {
                todo!("Evaluate expression")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{prelude::Color, ui::Val};

    use crate::guise::style::color::ColorValue;

    use super::*;

    #[test]
    fn test_serialize_color() {
        let value = StyleValue::Constant(ColorValue::Color(Color::RED));
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#""rgba(255, 0, 0, 1)""#);
    }

    #[test]
    fn test_deserialize_color() {
        let des =
            serde_json::from_str::<StyleValue<ColorValue>>(r#""rgba(255, 0, 0, 1)""#).unwrap();
        assert_eq!(des, StyleValue::Constant(ColorValue::Color(Color::RED)));
    }

    #[test]
    fn test_serialize_i32() {
        let value = StyleValue::Constant(1);
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#"1"#);
    }

    #[test]
    fn test_deserialize_i32() {
        let des = serde_json::from_str::<StyleValue<i32>>(r#"1"#).unwrap();
        assert_eq!(des, StyleValue::Constant(1));
    }

    #[test]
    fn test_serialize_f32() {
        let value = StyleValue::Constant(1.);
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#"1.0"#);
    }

    #[test]
    fn test_deserialize_f32() {
        let des = serde_json::from_str::<StyleValue<f32>>(r#"1.0"#).unwrap();
        assert_eq!(des, StyleValue::Constant(1.));
    }

    #[test]
    fn test_deserialize_f32_brief() {
        let des = serde_json::from_str::<StyleValue<f32>>(r#"1"#).unwrap();
        assert_eq!(des, StyleValue::Constant(1.));
    }

    #[test]
    fn test_serialize_length() {
        let value = StyleValue::Constant(Val::Px(10.));
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#"10.0"#);
    }

    #[test]
    fn test_deserialize_length_bare() {
        let des = serde_json::from_str::<StyleValue<Val>>(r#"10"#).unwrap();
        assert_eq!(des, StyleValue::Constant(Val::Px(10.)));
    }

    #[test]
    fn test_deserialize_length_units() {
        let des = serde_json::from_str::<StyleValue<Val>>(r#""10px""#).unwrap();
        assert_eq!(des, StyleValue::Constant(Val::Px(10.)));
        let des_pct = serde_json::from_str::<StyleValue<Val>>(r#""10%""#).unwrap();
        assert_eq!(des_pct, StyleValue::Constant(Val::Percent(10.)));
        let des_vw = serde_json::from_str::<StyleValue<Val>>(r#""10vw""#).unwrap();
        assert_eq!(des_vw, StyleValue::Constant(Val::Vw(10.)));
        let des_no_unit = serde_json::from_str::<StyleValue<Val>>(r#""10""#).unwrap();
        assert_eq!(des_no_unit, StyleValue::Constant(Val::Px(10.)));
    }

    #[test]
    fn test_serialize_var() {
        let value = StyleValue::<i32>::Expr(Expr::Var("bg".to_string()));
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#""var(--bg)""#);
    }

    #[test]
    fn test_deserialize_var_str() {
        let des = serde_json::from_str::<StyleValue<i32>>(r#""var(--bg)""#).unwrap();
        assert_eq!(des, StyleValue::<i32>::Expr(Expr::Var("bg".to_string())));
    }

    // #[test]
    // fn test_serialize_expr() {
    //     let value = StyleValue::<i32>::Expr(StyleExpr::Name("foo".to_string()));
    //     let ser = serde_json::to_string(&value);
    //     assert_eq!(ser.unwrap(), r#"{"name":"foo"}"#);
    // }
}
