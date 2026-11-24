use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// What an exemplar's instances are, written by name in exemplar files' `kind` and SAGA's `for`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum InstanceKind {
    /// A character: the player, an NPC or a monster.
    Actor,
    /// Scenery on the tile grid, which the wall tool places.
    Wall,
    /// Scenery placed freely, as furniture is.
    Fixture,
    /// Something carried, worn or wielded.
    Item,
}

impl InstanceKind {
    /// Every kind, in the order the editor lists them.
    pub const ALL: [InstanceKind; 4] = [
        InstanceKind::Actor,
        InstanceKind::Wall,
        InstanceKind::Fixture,
        InstanceKind::Item,
    ];

    /// The kind's name, as files write it.
    pub const fn name(self) -> &'static str {
        match self {
            InstanceKind::Actor => "Actor",
            InstanceKind::Wall => "Wall",
            InstanceKind::Fixture => "Fixture",
            InstanceKind::Item => "Item",
        }
    }
}

impl fmt::Display for InstanceKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A name that is no instance kind.
#[derive(Debug, PartialEq, Eq)]
pub struct UnknownInstanceKind;

impl FromStr for InstanceKind {
    type Err = UnknownInstanceKind;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        InstanceKind::ALL
            .into_iter()
            .find(|kind| kind.name() == name)
            .ok_or(UnknownInstanceKind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for kind in InstanceKind::ALL {
            assert_eq!(kind.name().parse(), Ok(kind));
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.name()));
            assert_eq!(serde_json::from_str::<InstanceKind>(&json).unwrap(), kind);
        }
    }

    #[test]
    fn four_letter_codes_are_unknown() {
        assert_eq!("Fixt".parse::<InstanceKind>(), Err(UnknownInstanceKind));
        assert_eq!("Actr".parse::<InstanceKind>(), Err(UnknownInstanceKind));
    }
}
