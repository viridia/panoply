use serde::{Deserialize, Serialize};

use super::TemplateNodeRef;

/// Dynamically-typed expression for parameters that can be passed to a template Call.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// No value
    None,

    /// A boolean value
    Bool(bool),

    /// A number
    Number(f32),

    /// A string
    String(String),

    /// A template node reference
    Node(TemplateNodeRef),

    /// A list of param values
    List(Box<[ParamValue]>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_bool() {
        let des = serde_json::from_str::<ParamValue>(r#"false"#).unwrap();
        assert!(matches!(des, ParamValue::Bool(false)));
    }

    #[test]
    fn test_serialize_bool() {
        let ser = serde_json::to_string(&ParamValue::Bool(false));
        assert_eq!(ser.unwrap(), r#"false"#);
    }

    #[test]
    fn test_deserialize_number() {
        let des = serde_json::from_str::<ParamValue>(r#"1"#).unwrap();
        match des {
            ParamValue::Number(n) => {
                assert_eq!(n, 1.0);
            }
            _ => panic!("match failed"),
        }
    }

    #[test]
    fn test_serialize_number() {
        let ser = serde_json::to_string(&ParamValue::Number(1.0));
        assert_eq!(ser.unwrap(), r#"1.0"#);
    }

    #[test]
    fn test_deserialize_list() {
        let des = serde_json::from_str::<ParamValue>(r#"[false, true]"#).unwrap();
        assert!(matches!(des, ParamValue::List(_)));
    }

    #[test]
    fn test_serialize_list() {
        let ser = serde_json::to_string(&ParamValue::List(Box::from([
            ParamValue::Bool(true),
            ParamValue::Bool(false),
        ])));
        assert_eq!(ser.unwrap(), r#"[true,false]"#);
    }
}
