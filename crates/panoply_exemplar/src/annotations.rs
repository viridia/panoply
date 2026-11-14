//! What the editor's property panel reads from an aspect type beside its fields, as
//! `bevy::annotations` give a field its caption and step.

use bevy::reflect::{reflect_trait, Reflect};

/// The icon an aspect's pane shows beside its caption: an asset path, as
/// `#[reflect(@AspectIcon("editor/icons/lucide/lock.png"))]` on the aspect type.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct AspectIcon(pub &'static str);

/// The decimal places a number's input rounds to while it is dragged, as
/// `#[reflect(@Precision(2))]` on the field. Typed values are kept as typed.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct Precision(pub i32);

/// A few words standing for a value when its rows are collapsed, as "Box 0.3 × 1 × 0.2" for a
/// collider. Registered with `#[reflect(PropertySummary)]`; a type without one shows no summary.
#[reflect_trait]
pub trait PropertySummary {
    /// The summary, short enough for a header.
    fn property_summary(&self) -> String;
}
