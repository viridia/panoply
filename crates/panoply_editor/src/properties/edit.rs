//! What the property editor edits, and how its edits are made. A [`PropertyRoot`] is where a value
//! lives, named by a stable key (D49). The panel never writes a value: each widget emits a
//! [`PropertyEdit`], a root, a path within it and what to do there, and [`apply_property_edits`]
//! makes the edits after the panel has run and records each in the undo history. Then
//! [`InspectedValues`] takes a fresh copy of each value the panel shows, so the next frame's panel
//! draws what the edits left. Only the panel's walk, the edits and the roots touch reflection.
//!
//! What a root can do beyond [`PropertyRoot`] is a trait of its own, found on the root by a cast
//! through reflection (`#[reflect_trait]`), as [`AspectHolder`] is: a root that cannot do it
//! refuses the edit, and `PropertyRoot` stays the same for every kind of root.

use std::{any::Any, borrow::Cow, sync::Arc};

use bevy::{
    asset::AssetPath,
    prelude::*,
    reflect::{Access, ParsedPath, ReflectMut, ReflectPath},
};
use panoply_exemplar::{Aspect, Exemplar};

use crate::{
    mode_selector::EditorModeSettings,
    undo::{UndoAction, UndoStack},
    unsaved::UnsavedAssets,
};

/// Building a path a step at a time, as the panel walks into a value.
pub trait PathSteps {
    /// The path into the field `name` of the struct, or struct variant, at this one.
    fn with_field(&self, name: &'static str) -> ParsedPath;
    /// The path into field `index` of the tuple, tuple struct or tuple variant at this one.
    fn with_tuple(&self, index: usize) -> ParsedPath;
    /// The path into item `index` of the list at this one.
    fn with_item(&self, index: usize) -> ParsedPath;
}

impl PathSteps for ParsedPath {
    fn with_field(&self, name: &'static str) -> ParsedPath {
        then(self, Access::Field(name.into()))
    }

    fn with_tuple(&self, index: usize) -> ParsedPath {
        then(self, Access::TupleIndex(index))
    }

    fn with_item(&self, index: usize) -> ParsedPath {
        then(self, Access::ListIndex(index))
    }
}

fn then(path: &ParsedPath, access: Access<'static>) -> ParsedPath {
    let mut path = path.clone();
    path.0.push(access.into());
    path
}

/// Where an inspected value lives: an aspect of an exemplar, the exemplar itself, and later an
/// instance or a nav point. Each type is registered, so that what else it can do is found by a
/// cast (see the module's documentation).
pub trait PropertyRoot: Reflect {
    /// The value's name in the panel and in the undo history, as "SceneryColliders".
    fn caption(&self) -> String;

    /// A key naming the value, the same for the same value across reloads, never an `Entity`.
    fn key(&self) -> String;

    /// A copy of the value, or `None` if it is gone.
    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>>;

    /// Lends the value to `change`, keeping what it does unless it fails, and marks where the
    /// value is kept as unsaved.
    fn modify(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError>;
}

/// A root that holds aspects, which can be added and removed: an exemplar.
#[reflect_trait]
pub trait AspectHolder {
    /// Lends the aspects to `change`, keeping what it does unless it fails, and marks where they
    /// are kept as unsaved.
    fn modify_aspects(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut Vec<Box<dyn Aspect>>) -> Result<(), BevyError>,
    ) -> Result<(), BevyError>;
}

/// What an edit does, as the undo history names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// A new value for a field, or a new variant for an enum.
    Set,
    /// An optional field given its value: `Some` written at its `Option`.
    Add,
    /// An optional field's value taken away: `None` written at its `Option`.
    Remove,
    /// An item, its index given, put into a list.
    AddItem(usize),
    /// An item, its index given, taken out of a list.
    RemoveItem(usize),
    /// Aspects added to and taken from an [`AspectHolder`] root.
    Aspects,
}

impl EditKind {
    /// The kind's words in the undo history, around the root's caption and the path.
    fn label(self, field: &str) -> String {
        match self {
            EditKind::Set => format!("Set {field}"),
            EditKind::Add => format!("Add {field}"),
            EditKind::Remove => format!("Remove {field}"),
            EditKind::AddItem(index) => format!("Add {field} item {}", index + 1),
            EditKind::RemoveItem(index) => format!("Remove {field} item {}", index + 1),
            EditKind::Aspects => format!("Change {field}.aspects"),
        }
    }

    /// What the edit does to the value at its path, given `value` for a kind that writes or
    /// inserts one.
    fn op(self, value: Option<Box<dyn PartialReflect>>) -> Option<EditOp> {
        Some(match self {
            EditKind::Set | EditKind::Add | EditKind::Remove => EditOp::Write(value?),
            EditKind::AddItem(index) => EditOp::Insert(index, value?),
            EditKind::RemoveItem(index) => EditOp::Delete(index),
            EditKind::Aspects => return None,
        })
    }
}

/// What an edit does at its path, and what its undo does. Bevy's `apply` on a list never shortens
/// it, so items are put in and taken out by edits of their own.
enum EditOp {
    /// Writes a value over the one at the path.
    Write(Box<dyn PartialReflect>),
    /// Puts an item into the list at the path, at an index.
    Insert(usize, Box<dyn PartialReflect>),
    /// Takes the item at an index out of the list at the path.
    Delete(usize),
    /// Takes the aspects named (by short type path) out of an [`AspectHolder`] root, then puts
    /// the others in at their indices: an undone removal puts an aspect back where it was.
    Aspects {
        add: Vec<(usize, Box<dyn Aspect>)>,
        remove: Vec<String>,
    },
}

impl EditOp {
    /// Does the edit to the value at `path` within `root`, and returns the edit that undoes it.
    fn apply(&self, root: &mut dyn PartialReflect, path: &ParsedPath) -> Result<EditOp, BevyError> {
        let target = path
            .reflect_element_mut(root)
            .map_err(|error| error.to_string())?;
        match self {
            EditOp::Write(value) => {
                let replaced = target.reflect_clone()?.into_partial_reflect();
                target.try_apply(value.as_ref())?;
                Ok(EditOp::Write(replaced))
            }
            EditOp::Insert(index, item) => {
                let ReflectMut::List(list) = target.reflect_mut() else {
                    return Err(format!("{path} is not a list").into());
                };
                if *index > list.len() {
                    return Err(format!("{path} has no place {index}").into());
                }
                list.insert(*index, item.reflect_clone()?.into_partial_reflect());
                Ok(EditOp::Delete(*index))
            }
            EditOp::Delete(index) => {
                let ReflectMut::List(list) = target.reflect_mut() else {
                    return Err(format!("{path} is not a list").into());
                };
                if *index >= list.len() {
                    return Err(format!("{path} has no item {index}").into());
                }
                Ok(EditOp::Insert(*index, list.remove(*index)))
            }
            EditOp::Aspects { .. } => Err("an aspects edit has no path".into()),
        }
    }

    /// Does the edit to `aspects`, and returns the edit that undoes it: the removed aspects put
    /// back at their indices, the added ones taken out.
    fn apply_aspects(
        aspects: &mut Vec<Box<dyn Aspect>>,
        add: &[(usize, Box<dyn Aspect>)],
        remove: &[String],
    ) -> Result<EditOp, BevyError> {
        let mut removed = Vec::new();
        for name in remove {
            let index = aspects
                .iter()
                .position(|aspect| aspect.reflect_short_type_path() == name)
                .ok_or_else(|| format!("there is no {name} to remove"))?;
            removed.push((index, name.clone()));
        }
        // Taken out from the last, so the indices kept are those before any came out.
        removed.sort_by_key(|(index, _)| std::cmp::Reverse(*index));
        let mut undo_add: Vec<(usize, Box<dyn Aspect>)> = removed
            .iter()
            .map(|(index, _)| (*index, aspects.remove(*index)))
            .collect();
        undo_add.reverse();
        let mut undo_remove = Vec::new();
        let mut add: Vec<_> = add.iter().collect();
        add.sort_by_key(|(index, _)| *index);
        for (index, aspect) in add {
            let name = aspect.reflect_short_type_path();
            if aspects.iter().any(|had| had.reflect_short_type_path() == name) {
                return Err(format!("there is a {name} already").into());
            }
            aspects.insert((*index).min(aspects.len()), aspect.clone_boxed());
            undo_remove.push(name.to_string());
        }
        Ok(EditOp::Aspects {
            add: undo_add,
            remove: undo_remove,
        })
    }

    /// Does the edit to `root`'s value at `path`, and returns the edit that undoes it.
    fn apply_to(
        &self,
        root: &dyn PropertyRoot,
        world: &mut World,
        path: &ParsedPath,
    ) -> Result<EditOp, BevyError> {
        let mut undo = None;
        if let EditOp::Aspects { add, remove } = self {
            let registry = world.resource::<AppTypeRegistry>().clone();
            let registry = registry.read();
            let root: &dyn Reflect = root;
            let holder = registry
                .get_type_data::<ReflectAspectHolder>(Any::type_id(root))
                .and_then(|holder| holder.get(root))
                .ok_or("it holds no aspects")?;
            holder.modify_aspects(world, &mut |aspects| {
                undo = Some(Self::apply_aspects(aspects, add, remove)?);
                Ok(())
            })?;
        } else {
            root.modify(world, &mut |value| {
                undo = Some(self.apply(value, path)?);
                Ok(())
            })?;
        }
        undo.ok_or_else(|| "the edit was not made".into())
    }
}

/// An edit a widget asks for: a `kind` of edit at `path` within `root`'s value, which `op` does.
pub struct PropertyEdit {
    root: Arc<dyn PropertyRoot>,
    path: ParsedPath,
    kind: EditKind,
    op: EditOp,
}

/// The edits asked for this frame, made by [`apply_property_edits`].
#[derive(Resource, Default)]
pub struct PropertyEdits(Vec<PropertyEdit>);

impl PropertyEdits {
    /// Asks for `value` to be written at `path` within `root`'s value.
    pub fn set(
        &mut self,
        root: &Arc<dyn PropertyRoot>,
        path: ParsedPath,
        value: impl PartialReflect,
    ) {
        self.push(root, path, EditKind::Set, Some(Box::new(value)));
    }

    /// Asks for a `kind` of edit at `path` within `root`'s value, with `value` for the kinds that
    /// write or insert one.
    pub fn push(
        &mut self,
        root: &Arc<dyn PropertyRoot>,
        path: ParsedPath,
        kind: EditKind,
        value: Option<Box<dyn PartialReflect>>,
    ) {
        let Some(op) = kind.op(value) else {
            error!("{kind:?} of {}{path} needs a value", root.caption());
            return;
        };
        self.0.push(PropertyEdit {
            root: root.clone(),
            path,
            kind,
            op,
        });
    }

    /// Asks for the aspects `add` to be added to `root`, after those it has, and those named
    /// `remove` (by short type path) to be taken from it, as one edit.
    pub fn change_aspects(
        &mut self,
        root: &Arc<dyn PropertyRoot>,
        add: Vec<Box<dyn Aspect>>,
        remove: Vec<String>,
    ) {
        if add.is_empty() && remove.is_empty() {
            return;
        }
        self.0.push(PropertyEdit {
            root: root.clone(),
            path: ParsedPath::empty(),
            kind: EditKind::Aspects,
            // Appended: an index past the end inserts at the end.
            op: EditOp::Aspects {
                add: add.into_iter().map(|aspect| (usize::MAX, aspect)).collect(),
                remove,
            },
        });
    }
}

/// A count of primary-button presses: the edits of one drag, or of one value typed and committed,
/// share it, so they undo as one.
#[derive(Resource, Default)]
pub struct EditGesture(u32);

/// The values the panel shows this frame, each with its root, copied after the frame's edits.
#[derive(Resource, Default)]
pub struct InspectedValues {
    pub values: Vec<(Arc<dyn PropertyRoot>, Box<dyn PartialReflect>)>,
}

/// The roots of the values the panel shows, decided from the world each frame.
#[derive(Resource, Default)]
pub struct InspectedRoots {
    pub roots: Vec<Arc<dyn PropertyRoot>>,
}

/// An edit within a root, in the undo history: undoing it makes the edit that reverses it, which
/// takes its place, so redo undoes that in turn. Its `kind` names it either way.
struct PropertyAction {
    root: Arc<dyn PropertyRoot>,
    path: ParsedPath,
    kind: EditKind,
    op: EditOp,
    gesture: u32,
    /// Its name in the undo history, taken from the edit as first made: `op` turns each way.
    label: String,
}

/// An edit's name in the undo history, from `op`, the edit as asked for: "Set LightSource.radius",
/// "Add SceneryColliders item 2", the path's field names, leaving out the steps into an `Option`,
/// a newtype or a list, which the panel does not show either. An aspects edit that adds or removes
/// one aspect names it: "Remove Marcus.Skin".
fn edit_label(root: &dyn PropertyRoot, path: &ParsedPath, kind: EditKind, op: &EditOp) -> String {
    let mut field = root.caption();
    if let EditOp::Aspects { add, remove } = op {
        match (add.as_slice(), remove.as_slice()) {
            ([(_, aspect)], []) => {
                return format!("Add {field}.{}", aspect.reflect_short_type_path())
            }
            ([], [name]) => return format!("Remove {field}.{name}"),
            _ => {}
        }
    }
    for step in &path.0 {
        if let Access::Field(name) = &step.access {
            field.push('.');
            field.push_str(name);
        }
    }
    kind.label(&field)
}

impl UndoAction for PropertyAction {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        match self.op.apply_to(self.root.as_ref(), world, &self.path) {
            Ok(op) => Box::new(PropertyAction { op, ..*self }),
            Err(error) => {
                error!("Could not undo or redo {}: {error}", self.label);
                self
            }
        }
    }

    fn label(&self) -> Cow<'static, str> {
        self.label.clone().into()
    }

    /// The edits of one drag, writing the same field, undo as one.
    fn coalesces_into(&self, earlier: &dyn UndoAction) -> bool {
        (earlier as &dyn Any)
            .downcast_ref::<PropertyAction>()
            .is_some_and(|earlier| {
                matches!(self.kind, EditKind::Set | EditKind::Add | EditKind::Remove)
                    && earlier.gesture == self.gesture
                    && earlier.kind == self.kind
                    && earlier.path == self.path
                    && earlier.root.key() == self.root.key()
            })
    }
}

/// Counts primary-button presses into [`EditGesture`].
pub fn count_edit_gestures(
    buttons: Res<ButtonInput<MouseButton>>,
    mut gesture: ResMut<EditGesture>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        gesture.0 = gesture.0.wrapping_add(1);
    }
}

/// Makes the frame's edits, each recorded in the undo history in the current mode.
pub fn apply_property_edits(world: &mut World) {
    let edits = std::mem::take(&mut world.resource_mut::<PropertyEdits>().0);
    if edits.is_empty() {
        return;
    }
    let gesture = world.resource::<EditGesture>().0;
    let mode = world.resource::<EditorModeSettings>().editor_mode;
    for PropertyEdit {
        root,
        path,
        kind,
        op,
    } in edits
    {
        let label = edit_label(root.as_ref(), &path, kind, &op);
        match op.apply_to(root.as_ref(), world, &path) {
            Ok(undo) => world.resource_mut::<UndoStack>().push(
                mode,
                PropertyAction {
                    root,
                    path,
                    kind,
                    op: undo,
                    gesture,
                    label,
                },
            ),
            Err(error) => error!("{label} failed: {error}"),
        }
    }
}

/// Copies the value of each of [`InspectedRoots`] into [`InspectedValues`].
pub fn read_inspected_values(world: &mut World) {
    let values: Vec<_> = world
        .resource::<InspectedRoots>()
        .roots
        .iter()
        .filter_map(|root| Some((root.clone(), root.read(world)?)))
        .collect();
    world.resource_mut::<InspectedValues>().values = values;
}

/// An aspect of an exemplar, named by the exemplar's asset path and the aspect's short type path.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct ExemplarAspect {
    pub exemplar: AssetPath<'static>,
    pub aspect: String,
}

impl ExemplarAspect {
    fn exemplar_id(&self, world: &World) -> Option<AssetId<Exemplar>> {
        world
            .resource::<AssetServer>()
            .get_path_id(&self.exemplar)?
            .try_typed::<Exemplar>()
            .ok()
    }
}

impl PropertyRoot for ExemplarAspect {
    fn caption(&self) -> String {
        self.aspect.clone()
    }

    fn key(&self) -> String {
        format!("{} {}", self.exemplar, self.aspect)
    }

    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let exemplar = world
            .resource::<Assets<Exemplar>>()
            .get(self.exemplar_id(world)?)?;
        let aspect = exemplar
            .0
            .aspects
            .iter()
            .find(|aspect| aspect.reflect_short_type_path() == self.aspect)?;
        Some(aspect.clone_boxed().into_partial_reflect())
    }

    fn modify(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        let id = self
            .exemplar_id(world)
            .ok_or_else(|| format!("{} is not loaded", self.exemplar))?;
        let catalog = world
            .resource::<AssetServer>()
            .get_path_id(self.exemplar.without_label())
            .ok_or_else(|| format!("{} has no catalog", self.exemplar))?;
        world.resource_scope(|_, mut exemplars: Mut<Assets<Exemplar>>| {
            Exemplar::edit(&mut exemplars, id, |data| {
                let aspect = data
                    .aspects
                    .iter_mut()
                    .find(|aspect| aspect.reflect_short_type_path() == self.aspect)
                    .ok_or_else(|| format!("{} has no {}", self.exemplar, self.aspect))?;
                change(aspect.as_partial_reflect_mut())
            })
        })?;
        world.resource_mut::<UnsavedAssets>().mark(catalog);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bevy::reflect::{ReflectRef, TypeRegistry};

    use super::*;
    use crate::properties::option_value;

    #[derive(Reflect, Clone, Default, Debug, PartialEq)]
    struct Size {
        width: f32,
        height: f32,
        depth: Option<f32>,
        parts: Vec<f32>,
    }

    #[derive(Resource, Default)]
    struct Held(Size);

    /// `text` as a path.
    fn path(text: &str) -> ParsedPath {
        ParsedPath::parse(text).unwrap()
    }

    /// The [`Size`] in [`Held`].
    #[derive(Reflect)]
    struct HeldRoot;

    impl PropertyRoot for HeldRoot {
        fn caption(&self) -> String {
            "Size".into()
        }

        fn key(&self) -> String {
            "held".into()
        }

        fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
            Some(Box::new(world.resource::<Held>().0.clone()))
        }

        fn modify(
            &self,
            world: &mut World,
            change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
        ) -> Result<(), BevyError> {
            change(&mut world.resource_mut::<Held>().0)
        }
    }

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<Held>();
        world.init_resource::<PropertyEdits>();
        world.init_resource::<EditGesture>();
        world.init_resource::<UndoStack>();
        world.init_resource::<EditorModeSettings>();
        world
    }

    /// Asks for `width` and applies it, in gesture `gesture`.
    fn edit_width(world: &mut World, gesture: u32, width: f32) {
        world.resource_mut::<EditGesture>().0 = gesture;
        let root: Arc<dyn PropertyRoot> = Arc::new(HeldRoot);
        world
            .resource_mut::<PropertyEdits>()
            .set(&root, path(".width"), width);
        apply_property_edits(world);
    }

    fn width(world: &World) -> f32 {
        world.resource::<Held>().0.width
    }

    fn undo(world: &mut World) {
        world.resource_scope(|world, mut stack: Mut<UndoStack>| stack.undo(world));
    }

    fn redo(world: &mut World) {
        world.resource_scope(|world, mut stack: Mut<UndoStack>| stack.redo(world));
    }

    #[test]
    fn an_edit_is_made_and_undone() {
        let mut world = world();
        edit_width(&mut world, 1, 2.);
        assert_eq!(width(&world), 2.);
        undo(&mut world);
        assert_eq!(width(&world), 0.);
        redo(&mut world);
        assert_eq!(width(&world), 2.);
    }

    #[test]
    fn a_drag_undoes_to_before_it_began() {
        let mut world = world();
        edit_width(&mut world, 1, 1.);
        edit_width(&mut world, 2, 2.);
        edit_width(&mut world, 2, 3.);
        edit_width(&mut world, 2, 4.);
        undo(&mut world);
        assert_eq!(width(&world), 1.);
        undo(&mut world);
        assert_eq!(width(&world), 0.);
    }

    #[test]
    fn edits_of_other_fields_do_not_coalesce() {
        let mut world = world();
        edit_width(&mut world, 1, 2.);
        let root: Arc<dyn PropertyRoot> = Arc::new(HeldRoot);
        world
            .resource_mut::<PropertyEdits>()
            .set(&root, path(".height"), 5.0f32);
        apply_property_edits(&mut world);
        undo(&mut world);
        assert_eq!(
            world.resource::<Held>().0,
            Size {
                width: 2.,
                height: 0.,
                depth: None,
                parts: Vec::new(),
            }
        );
    }

    /// Asks for `depth` to be added or removed, with the value the panel would build, and
    /// applies it.
    fn add_or_remove_depth(world: &mut World, gesture: u32, kind: EditKind) {
        world.resource_mut::<EditGesture>().0 = gesture;
        let mut registry = TypeRegistry::default();
        registry.register::<Size>();
        let info = <Option<f32> as bevy::reflect::Typed>::type_info();
        let value = option_value(&registry, info, kind == EditKind::Add).unwrap();
        let root: Arc<dyn PropertyRoot> = Arc::new(HeldRoot);
        world
            .resource_mut::<PropertyEdits>()
            .push(&root, path(".depth"), kind, Some(value));
        apply_property_edits(world);
    }

    fn depth(world: &World) -> Option<f32> {
        world.resource::<Held>().0.depth
    }

    #[test]
    fn an_optional_field_is_added_and_removed() {
        let mut world = world();
        add_or_remove_depth(&mut world, 1, EditKind::Add);
        assert_eq!(depth(&world), Some(0.));
        add_or_remove_depth(&mut world, 2, EditKind::Remove);
        assert_eq!(depth(&world), None);
        undo(&mut world);
        assert_eq!(depth(&world), Some(0.));
        undo(&mut world);
        assert_eq!(depth(&world), None);
        redo(&mut world);
        assert_eq!(depth(&world), Some(0.));
    }

    /// Asks for a list edit of `parts`, and applies it.
    fn edit_parts(world: &mut World, kind: EditKind, value: Option<f32>) {
        let root: Arc<dyn PropertyRoot> = Arc::new(HeldRoot);
        let value = value.map(|value| Box::new(value) as Box<dyn PartialReflect>);
        world
            .resource_mut::<PropertyEdits>()
            .push(&root, path(".parts"), kind, value);
        apply_property_edits(world);
    }

    fn parts(world: &World) -> Vec<f32> {
        world.resource::<Held>().0.parts.clone()
    }

    #[test]
    fn list_items_are_added_and_removed() {
        let mut world = world();
        world.resource_mut::<Held>().0.parts = vec![1., 2.];
        edit_parts(&mut world, EditKind::AddItem(2), Some(3.));
        assert_eq!(parts(&world), [1., 2., 3.]);
        edit_parts(&mut world, EditKind::RemoveItem(0), None);
        assert_eq!(parts(&world), [2., 3.]);
        undo(&mut world);
        assert_eq!(parts(&world), [1., 2., 3.]);
        undo(&mut world);
        assert_eq!(parts(&world), [1., 2.]);
        redo(&mut world);
        redo(&mut world);
        assert_eq!(parts(&world), [2., 3.]);
    }

    #[test]
    fn a_failed_edit_is_not_in_the_history() {
        let mut world = world();
        edit_width(&mut world, 1, 2.);
        edit_parts(&mut world, EditKind::RemoveItem(0), None);
        assert!(parts(&world).is_empty());
        undo(&mut world);
        assert_eq!(width(&world), 0.);
    }

    #[derive(Reflect, Clone, Debug, PartialEq)]
    enum Shape {
        Box { half_extents: Vec3 },
        Ramp { half_extents: Vec3, facing: f32 },
        Sphere { radius: f32 },
        Ball(f32),
    }

    /// `shape` as its variant `name`, as the panel would build it.
    fn switched(shape: Shape, name: &str) -> Shape {
        let mut registry = TypeRegistry::default();
        registry.register::<Shape>();
        registry.register::<Vec3>();
        registry.register::<f32>();
        let ReflectRef::Enum(current) = shape.reflect_ref() else {
            unreachable!()
        };
        let value = crate::properties::variant_value(&registry, current, name).unwrap();
        Shape::from_reflect(value.as_ref()).unwrap()
    }

    #[test]
    fn a_new_variant_keeps_the_fields_it_shares() {
        let half_extents = Vec3::new(1., 2., 3.);
        assert_eq!(
            switched(Shape::Box { half_extents }, "Ramp"),
            Shape::Ramp {
                half_extents,
                facing: 0.
            }
        );
        assert_eq!(
            switched(Shape::Box { half_extents }, "Sphere"),
            Shape::Sphere { radius: 0. }
        );
        assert_eq!(switched(Shape::Ball(2.), "Sphere"), Shape::Sphere { radius: 0. });
        assert_eq!(switched(Shape::Sphere { radius: 2. }, "Ball"), Shape::Ball(0.));
    }

    #[test]
    fn the_label_names_the_field() {
        let label = |text: &str, kind| {
            let op = EditOp::Write(Box::new(0.0f32));
            edit_label(&HeldRoot, &path(text), kind, &op)
        };
        assert_eq!(label(".width", EditKind::Set), "Set Size.width");
        assert_eq!(label(".0.radius.0", EditKind::Set), "Set Size.radius");
        assert_eq!(label(".depth", EditKind::Add), "Add Size.depth");
        assert_eq!(label(".0", EditKind::RemoveItem(1)), "Remove Size item 2");
        assert_eq!(label(".0[1].shape", EditKind::Set), "Set Size.shape");
    }

    #[derive(Component, Reflect, Clone, Default, Debug, PartialEq)]
    struct Wings(f32);

    #[derive(Component, Reflect, Clone, Default, Debug, PartialEq)]
    struct Horns;

    #[derive(Component, Reflect, Clone, Default, Debug, PartialEq)]
    struct Tail;

    macro_rules! test_aspect {
        ($type:ident) => {
            impl Aspect for $type {
                fn name(&self) -> &str {
                    stringify!($type)
                }

                fn can_attach(&self, _meta_type: panoply_exemplar::InstanceType) -> bool {
                    true
                }

                fn attach(
                    &self,
                    entity: &mut EntityWorldMut,
                ) -> &'static dyn panoply_exemplar::DetachAspect {
                    static DETACH: panoply_exemplar::RemoveComponent<$type> =
                        panoply_exemplar::RemoveComponent::<$type>::new();
                    entity.insert(self.clone());
                    &DETACH
                }

                fn clone_boxed(&self) -> Box<dyn Aspect> {
                    Box::new(self.clone())
                }
            }
        };
    }

    test_aspect!(Wings);
    test_aspect!(Horns);
    test_aspect!(Tail);

    #[derive(Resource, Default)]
    struct HeldAspects(Vec<Box<dyn Aspect>>);

    /// The aspects in [`HeldAspects`], as an exemplar named Marcus.
    #[derive(Reflect)]
    #[reflect(AspectHolder)]
    struct AspectsRoot;

    impl PropertyRoot for AspectsRoot {
        fn caption(&self) -> String {
            "Marcus".into()
        }

        fn key(&self) -> String {
            "aspects".into()
        }

        fn read(&self, _world: &World) -> Option<Box<dyn PartialReflect>> {
            None
        }

        fn modify(
            &self,
            _world: &mut World,
            _change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
        ) -> Result<(), BevyError> {
            Err("no fields".into())
        }
    }

    impl AspectHolder for AspectsRoot {
        fn modify_aspects(
            &self,
            world: &mut World,
            change: &mut dyn FnMut(&mut Vec<Box<dyn Aspect>>) -> Result<(), BevyError>,
        ) -> Result<(), BevyError> {
            change(&mut world.resource_mut::<HeldAspects>().0)
        }
    }

    /// A world holding Wings, Horns and Tail, in that order, with the roots registered.
    fn aspects_world() -> World {
        let mut world = world();
        world.insert_resource(HeldAspects(vec![
            Box::new(Wings(2.)),
            Box::new(Horns),
            Box::new(Tail),
        ]));
        world.init_resource::<AppTypeRegistry>();
        {
            let registry = world.resource::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<AspectsRoot>();
            registry.register::<HeldRoot>();
        }
        world
    }

    fn held_aspects(world: &World) -> Vec<String> {
        world
            .resource::<HeldAspects>()
            .0
            .iter()
            .map(|aspect| aspect.reflect_short_type_path().to_string())
            .collect()
    }

    fn change_aspects(world: &mut World, root: impl PropertyRoot, add: Vec<Box<dyn Aspect>>, remove: &[&str]) {
        let root: Arc<dyn PropertyRoot> = Arc::new(root);
        let remove = remove.iter().map(|name| name.to_string()).collect();
        world
            .resource_mut::<PropertyEdits>()
            .change_aspects(&root, add, remove);
        apply_property_edits(world);
    }

    fn last_label(world: &World) -> String {
        world.resource::<UndoStack>().latest_label().unwrap()
    }

    #[test]
    fn aspects_are_added_and_removed_and_undo_in_place() {
        let mut world = aspects_world();
        change_aspects(&mut world, AspectsRoot, Vec::new(), &["Horns"]);
        assert_eq!(held_aspects(&world), ["Wings", "Tail"]);
        assert_eq!(last_label(&world), "Remove Marcus.Horns");
        undo(&mut world);
        assert_eq!(held_aspects(&world), ["Wings", "Horns", "Tail"]);
        assert_eq!(world.resource::<HeldAspects>().0[0].reflect_short_type_path(), "Wings");
        redo(&mut world);
        assert_eq!(held_aspects(&world), ["Wings", "Tail"]);

        // Several at once, one undo entry; an added aspect goes last.
        change_aspects(&mut world, AspectsRoot, vec![Box::new(Horns)], &["Wings", "Tail"]);
        assert_eq!(held_aspects(&world), ["Horns"]);
        assert_eq!(last_label(&world), "Change Marcus.aspects");
        undo(&mut world);
        assert_eq!(held_aspects(&world), ["Wings", "Tail"]);
        undo(&mut world);
        assert_eq!(held_aspects(&world), ["Wings", "Horns", "Tail"]);
    }

    #[test]
    fn an_aspect_already_held_is_not_added_again() {
        let mut world = aspects_world();
        change_aspects(&mut world, AspectsRoot, vec![Box::new(Tail)], &[]);
        assert_eq!(held_aspects(&world), ["Wings", "Horns", "Tail"]);
        assert!(world.resource::<UndoStack>().latest_label().is_none());
    }

    #[test]
    fn a_root_without_aspects_refuses_an_aspects_edit() {
        let mut world = aspects_world();
        change_aspects(&mut world, HeldRoot, Vec::new(), &["Horns"]);
        assert_eq!(held_aspects(&world), ["Wings", "Horns", "Tail"]);
        assert!(world.resource::<UndoStack>().latest_label().is_none());
    }
}
