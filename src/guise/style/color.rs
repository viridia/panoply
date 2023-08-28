use bevy::prelude::Color;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de::Visitor, Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// A CSS-style color
#[derive(Debug, Clone, PartialEq, Default, Copy)]
pub enum ColorValue {
    /// "transparent" means "skip drawing entirely"
    #[default]
    Transparent,

    /// An actual color
    Color(Color),
    // TODO: We might store the source format (hex, rgba) for round-tripping serialization.
}

impl ColorValue {
    pub fn is_transparent(&self) -> bool {
        matches!(self, Self::Transparent)
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Transparent => Color::NONE,
            Self::Color(c) => *c,
        }
    }
}

impl fmt::Display for ColorValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transparent => write!(f, "transparent"),
            Self::Color(color) => match color {
                Color::Rgba {
                    red,
                    green,
                    blue,
                    alpha,
                } => write!(
                    f,
                    "rgba({}, {}, {}, {})",
                    red * 255.0,
                    green * 255.0,
                    blue * 255.0,
                    alpha
                ),

                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha,
                } => write!(f, "hsla({}, {}, {}, {})", hue, saturation, lightness, alpha),

                _ => {
                    panic!("Unsupported color format")
                }
            },
        }
    }
}

impl Serialize for ColorValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ColorValueVisitor)
    }
}

struct ColorValueVisitor;

impl<'de> Visitor<'de> for ColorValueVisitor {
    type Value = ColorValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("CSS color")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if s == "transparent" {
            return Ok(ColorValue::Transparent);
        }

        let h = Color::hex(s);
        if h.is_ok() {
            return Ok(ColorValue::Color(h.unwrap()));
        }

        // TODO: Colors by name

        lazy_static! {
            static ref RE_RGBA: Regex =
                Regex::new(r"^rgba\(([\d\.]+),\s*([\d\.]+),\s*([\d\.]+),\s*([\d\.]+)\)$").unwrap();
            static ref RE_HSLA: Regex =
                Regex::new(r"^hsla\(([\d\.]+),\s*([\d\.]+),\s*([\d\.]+),\s*([\d\.]+)\)$").unwrap();
        }

        RE_RGBA
            .captures(&s)
            .map(|cap| {
                ColorValue::Color(Color::rgba(
                    f32::from_str(&cap[1]).unwrap() / 255.0,
                    f32::from_str(&cap[2]).unwrap() / 255.0,
                    f32::from_str(&cap[3]).unwrap() / 255.0,
                    f32::from_str(&cap[4]).unwrap(),
                ))
            })
            .or(RE_HSLA.captures(&s).map(|cap| {
                ColorValue::Color(Color::hsla(
                    f32::from_str(&cap[1]).unwrap() / 360.0,
                    f32::from_str(&cap[2]).unwrap() / 100.0,
                    f32::from_str(&cap[3]).unwrap() / 100.0,
                    f32::from_str(&cap[4]).unwrap(),
                ))
            }))
            .ok_or(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(&s),
                &"Invalid color format",
            ))
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Color;

    use super::*;

    #[test]
    fn test_serialize_rgba() {
        let value = ColorValue::Color(Color::RED);
        let ser = serde_json::to_string(&value).unwrap();
        assert_eq!(ser, r#""rgba(255, 0, 0, 1)""#);
    }

    #[test]
    fn test_deserialize_hex() {
        assert_eq!(
            serde_json::from_str::<ColorValue>(&"\"#ff0000ff\"").unwrap(),
            ColorValue::Color(Color::rgba(1., 0., 0., 1.))
        );
        assert_eq!(
            serde_json::from_str::<ColorValue>(&"\"#ff0000\"").unwrap(),
            ColorValue::Color(Color::rgba(1., 0., 0., 1.))
        );
        assert_eq!(
            serde_json::from_str::<ColorValue>(&"\"#f00f\"").unwrap(),
            ColorValue::Color(Color::rgba(1., 0., 0., 1.))
        );
        assert_eq!(
            serde_json::from_str::<ColorValue>(&"\"#f00\"").unwrap(),
            ColorValue::Color(Color::rgba(1., 0., 0., 1.))
        );
    }

    #[test]
    fn test_deserialize_rgba() {
        assert_eq!(
            serde_json::from_str::<ColorValue>(&"\"rgba(255, 0, 0, 1)\"").unwrap(),
            ColorValue::Color(Color::rgba(1., 0., 0., 1.))
        );
    }

    #[test]
    fn test_serialize_transparent() {
        let value = ColorValue::Transparent;
        let ser = serde_json::to_string(&value).unwrap();
        assert_eq!(ser, r#""transparent""#);
    }

    #[test]
    fn test_deserialize_transparent() {
        let value = ColorValue::Transparent;
        let ser = serde_json::to_string(&value).unwrap();
        let des = serde_json::from_str::<ColorValue>(&ser).unwrap();
        assert_eq!(des, value);
    }
}
