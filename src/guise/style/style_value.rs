use serde::{Deserialize, Serialize};

// use super::color::ColorValue;

/// Enum representing possible values of a style attribute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StyleValue<T> {
    // TODO: Possible other choices, assuming we can figure out how to serialize them.
    // Unset,
    // Initial
    // Inherit
    //
    /// The value has been set to a constant
    Constant(T),

    /// A reference to a named style variable. The variable must be of the
    /// correct type (string, bool, int, float, Val, Rect, etc.)
    Var(VarRef),

    /// The value has been set to an evaluable expression.
    Expr(StyleExpr),
}

/// Reference a style variable by name
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarRef {
    var: String,
}

/// A style expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StyleExpr {
    #[serde(rename = "name")]
    Name(String),
    Add(String),
    // Expression ops:
    // Add
    // Sub
    // Mul
    // Div
    // Concat
    // Format
    // And
    // Or
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Color;

    use crate::guise::style::color::ColorValue;

    use super::*;

    #[test]
    fn test_serialize_constant_color() {
        let value = StyleValue::Constant(ColorValue::Color(Color::RED));
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#""rgba(255, 0, 0, 1)""#);
    }

    #[test]
    fn test_deserialize_constant_color() {
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
    fn test_serialize_var() {
        let value = StyleValue::Constant(StyleValue::<String>::Var(VarRef {
            var: "bg".to_string(),
        }));
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#"{"var":"bg"}"#);
    }

    #[test]
    fn test_deserialize_var() {
        let des = serde_json::from_str::<StyleValue<String>>(r#"{"var":"bg"}"#).unwrap();
        assert_eq!(
            des,
            StyleValue::<String>::Var(VarRef {
                var: "bg".to_string()
            })
        );
    }

    #[test]
    fn test_serialize_expr() {
        let value = StyleValue::Constant(StyleValue::<String>::Expr(StyleExpr::Name(
            "foo".to_string(),
        )));
        let ser = serde_json::to_string(&value);
        assert_eq!(ser.unwrap(), r#"{"name":"foo"}"#);
    }
}