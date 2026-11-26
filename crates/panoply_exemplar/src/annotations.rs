//! What the editor's property panel reads from an aspect type beside its fields, as
//! `bevy::annotations` give a field its caption and step. A scripted aspect gets the same
//! annotations from its attributes ([`add_editor_attributes`]).

use std::sync::Mutex;

use bevy::{
    annotations::{ReadOnly, Step, ValueRange},
    asset::{AssetId, UntypedHandle},
    ecs::world::World,
    platform::collections::HashSet,
    reflect::{reflect_trait, Reflect},
};
use panoply_saga::{ExprType, HostState};

use crate::Exemplar;

/// Registers on `host` the attributes a scripted aspect and its fields may take, each the
/// annotation a native aspect gives itself with `#[reflect(@...)]`: `#[icon("path")]` on the
/// aspect, and `#[range(min, max)]`, `#[step(step)]`, `#[precision(places)]` and `#[read_only]`
/// on a field. [`ExemplarPlugin`](crate::ExemplarPlugin) registers them on the game's host.
pub fn add_editor_attributes(host: &mut HostState) {
    fn number(args: &[Box<dyn Reflect>], index: usize) -> f64 {
        args[index]
            .downcast_ref::<f64>()
            .copied()
            .unwrap_or_default()
    }
    host.add_aspect_attribute("icon", vec![ExprType::String], |builder, args| {
        let path = args[0]
            .downcast_ref::<String>()
            .map_or("", |path| intern(path));
        builder.attribute(AspectIcon(path))
    });
    host.add_field_attribute(
        "range",
        vec![ExprType::F64, ExprType::F64],
        |builder, args| builder.attribute(ValueRange(number(args, 0)..=number(args, 1))),
    );
    host.add_field_attribute("step", vec![ExprType::F64], |builder, args| {
        builder.attribute(Step(number(args, 0)))
    });
    host.add_field_attribute("precision", vec![ExprType::I32], |builder, args| {
        let places = args[0].downcast_ref::<i32>().copied().unwrap_or_default();
        builder.attribute(Precision(places))
    });
    host.add_field_attribute("read_only", Vec::new(), |builder, _| {
        builder.attribute(ReadOnly)
    });
}

/// `text`, kept for good, for what takes a `&'static str` but is named at runtime: a scripted
/// aspect's icon, or its fields' names in the editor's rows. The same text is kept once, so a
/// module reloaded again and again keeps nothing more.
pub fn intern(text: &str) -> &'static str {
    static INTERNED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut interned = INTERNED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let interned = interned.get_or_insert_with(HashSet::default);
    if let Some(&kept) = interned.get(text) {
        return kept;
    }
    let kept: &'static str = Box::leak(text.into());
    interned.insert(kept);
    kept
}

/// The icon an aspect's pane shows beside its caption: an asset path, as
/// `#[reflect(@AspectIcon("editor/icons/lucide/lock.png"))]` on the aspect type.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct AspectIcon(pub &'static str);

/// The decimal places a number's input rounds to while it is dragged, as
/// `#[reflect(@Precision(2))]` on the field. Typed values are kept as typed.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct Precision(pub i32);

/// The keys a map aspect may hold: the panel shows a row for each and no other key, such as one
/// per slot of an actor's `Features`. Registered with `#[reflect(PropertyMapKeys)]`; a map without
/// it shows its entries, each key typed.
#[reflect_trait]
pub trait PropertyMapKeys {
    /// The keys for the aspect as `exemplar` holds it, its chain and what it names read in
    /// `world`, or `None` while what they come from is still loading. A handle put in `held` keeps
    /// what it names loaded: the caller keeps it until it asks again.
    fn property_map_keys(
        &self,
        world: &World,
        exemplar: AssetId<Exemplar>,
        held: &mut Vec<UntypedHandle>,
    ) -> Option<Vec<String>>;
}

/// A few words standing for a value when its rows are collapsed, as "Box 0.3 × 1 × 0.2" for a
/// collider. Registered with `#[reflect(PropertySummary)]`; a type without one shows no summary.
#[reflect_trait]
pub trait PropertySummary {
    /// The summary, short enough for a header.
    fn property_summary(&self) -> String;
}
