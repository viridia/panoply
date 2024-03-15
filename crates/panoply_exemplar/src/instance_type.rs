use std::fmt::Write;

use serde::{Deserialize, Serialize};

/// A unique identifier for an instance meta-type (such as 'actor', 'fixture', etc.).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceType(u32);

impl InstanceType {
    /// A special instance type that represents no type.
    pub const NONE: InstanceType = InstanceType::new(0);

    /// Construct a new instance type from a raw identifier.
    pub const fn new(id: u32) -> Self {
        InstanceType(id)
    }

    /// Construct a new instance type from a 4-character string.
    pub const fn from_chars(id: [char; 4]) -> Self {
        if id.len() != 4 {
            panic!("InstanceType must be constructed from exactly 4 characters");
        }
        InstanceType(
            ((id[0] as u32) << 24)
                | ((id[1] as u32) << 16)
                | ((id[2] as u32) << 8)
                | (id[3] as u32),
        )
    }

    /// Construct a new instance type from a 4-character string.
    pub const fn from_str(id: &str) -> Self {
        let b = id.as_bytes();
        InstanceType(
            ((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | (b[3] as u32),
        )
    }

    /// Get the raw identifier for this instance type.
    pub const fn to_chars(&self) -> [char; 4] {
        [
            ((self.0 >> 24) & 0xFF) as u8 as char,
            ((self.0 >> 16) & 0xFF) as u8 as char,
            ((self.0 >> 8) & 0xFF) as u8 as char,
            (self.0 & 0xFF) as u8 as char,
        ]
    }
}

impl std::fmt::Display for InstanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_char(char::from_u32((self.0 >> 24) & 0xFF).unwrap())?;
        f.write_char(char::from_u32((self.0 >> 16) & 0xFF).unwrap())?;
        f.write_char(char::from_u32((self.0 >> 8) & 0xFF).unwrap())?;
        f.write_char(char::from_u32(self.0 & 0xFF).unwrap())
    }
}

impl std::fmt::Debug for InstanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#({})", self)
    }
}

impl Serialize for InstanceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InstanceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.len() != 4 {
            return Err(serde::de::Error::invalid_length(s.len(), &"4"));
        }
        let mut chars = [0 as char; 4];
        chars.copy_from_slice(&s.chars().collect::<Vec<char>>()[..4]);
        Ok(InstanceType::from_chars(chars))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM: InstanceType = InstanceType::from_chars(['I', 't', 'e', 'm']);
    const WALL: InstanceType = InstanceType::from_str("Wall");
    const WALL2: InstanceType = InstanceType::from_chars(['W', 'a', 'l', 'l']);

    #[test]
    fn test_construct() {
        assert_eq!(ITEM.to_chars(), ['I', 't', 'e', 'm']);
        assert_eq!(ITEM.to_string(), "Item");
        assert_eq!(WALL, WALL2);
    }

    #[test]
    fn test_serialize() {
        let json = serde_json::to_string(&ITEM).unwrap();
        assert_eq!(json, r#""Item""#);
    }

    #[test]
    fn test_deserialize() {
        let item: InstanceType = serde_json::from_str(r#""Item""#).unwrap();
        assert_eq!(item, ITEM);
    }
}
