//! What the property editor edits, and how its edits are made. A [`PropertyRoot`] is where a value
//! lives, named by a stable key (D49). The panel never writes a value: each widget emits a
//! [`PropertyEdit`], a root, a path within it and what to do there, and [`apply_property_edits`]
//! makes the edits after the panel has run and records each in the undo history. Then
//! [`InspectedValues`] takes a fresh copy of each value the panel shows, so the next frame's panel
//! draws what the edits left. Only the panel's walk, the edits and the roots touch reflection.

use std::{any::Any, borrow::Cow, sync::Arc};

use bevy::{
    asset::AssetPath,
    prelude::*,
    reflect::{ReflectMut, ReflectPath},
};
use panoply_exemplar::Exemplar;

use crate::{
    mode_selector::EditorModeSettings,
    undo::{UndoAction, UndoStack},
    unsaved::UnsavedAssets,
};

/// Where an inspected value lives: an aspect of an exemplar, and later an instance or a nav point.
pub trait PropertyRoot: Any + Send + Sync {
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
        }
    }

    /// What the edit does to the value at its path, given `value` for a kind that writes or
    /// inserts one.
    fn op(self, value: Option<Box<dyn PartialReflect>>) -> Option<EditOp> {
        Some(match self {
            EditKind::Set | EditKind::Add | EditKind::Remove => EditOp::Write(value?),
            EditKind::AddItem(index) => EditOp::Insert(index, value?),
            EditKind::RemoveItem(index) => EditOp::Delete(index),
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
}

impl EditOp {
    /// Does the edit to the value at `path` within `root`, and returns the edit that undoes it.
    fn apply(&self, root: &mut dyn PartialReflect, path: &str) -> Result<EditOp, BevyError> {
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
        }
    }

    /// Does the edit to `root`'s value at `path`, and returns the edit that undoes it.
    fn apply_to(
        &self,
        root: &dyn PropertyRoot,
        world: &mut World,
        path: &str,
    ) -> Result<EditOp, BevyError> {
        let mut undo = None;
        root.modify(world, &mut |value| {
            undo = Some(self.apply(value, path)?);
            Ok(())
        })?;
        undo.ok_or_else(|| "the edit was not made".into())
    }
}

/// An edit a widget asks for: a `kind` of edit at `path` within `root`'s value, writing or
/// inserting `value` for the kinds that need one.
pub struct PropertyEdit {
    pub root: Arc<dyn PropertyRoot>,
    pub path: String,
    pub kind: EditKind,
    pub value: Option<Box<dyn PartialReflect>>,
}

/// The edits asked for this frame, made by [`apply_property_edits`].
#[derive(Resource, Default)]
pub struct PropertyEdits(pub Vec<PropertyEdit>);

impl PropertyEdits {
    /// Asks for `value` to be written at `path` within `root`'s value.
    pub fn set(&mut self, root: &Arc<dyn PropertyRoot>, path: String, value: impl PartialReflect) {
        self.push(root, path, EditKind::Set, Some(Box::new(value)));
    }

    /// Asks for a `kind` of edit at `path` within `root`'s value, with `value` for the kinds that
    /// write or insert one.
    pub fn push(
        &mut self,
        root: &Arc<dyn PropertyRoot>,
        path: String,
        kind: EditKind,
        value: Option<Box<dyn PartialReflect>>,
    ) {
        self.0.push(PropertyEdit {
            root: root.clone(),
            path,
            kind,
            value,
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
    path: String,
    kind: EditKind,
    op: EditOp,
    gesture: u32,
}

impl UndoAction for PropertyAction {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        match self.op.apply_to(self.root.as_ref(), world, &self.path) {
            Ok(op) => Box::new(PropertyAction { op, ..*self }),
            Err(error) => {
                error!("Could not undo or redo {}: {error}", self.label());
                self
            }
        }
    }

    /// "Set LightSource.radius", "Add SceneryColliders item 2": the path's field names, leaving
    /// out the steps into an `Option`, a newtype or a list, which the panel does not show either.
    fn label(&self) -> Cow<'static, str> {
        let mut field = self.root.caption();
        for name in self.path.split('.') {
            let name = name.split('[').next().unwrap_or_default();
            if !name.is_empty() && !name.bytes().all(|byte| byte.is_ascii_digit()) {
                field.push('.');
                field.push_str(name);
            }
        }
        self.kind.label(&field).into()
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
        value,
    } in edits
    {
        let Some(op) = kind.op(value) else {
            error!("{kind:?} of {}{path} needs a value", root.caption());
            continue;
        };
        match op.apply_to(root.as_ref(), world, &path) {
            Ok(undo) => world.resource_mut::<UndoStack>().push(
                mode,
                PropertyAction {
                    root,
                    path,
                    kind,
                    op: undo,
                    gesture,
                },
            ),
            Err(error) => error!("Could not edit {}{path}: {error}", root.caption()),
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
#[derive(Debug, Clone, PartialEq)]
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

    /// The [`Size`] in [`Held`].
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
            .set(&root, ".width".into(), width);
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
            .set(&root, ".height".into(), 5.0f32);
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
            .push(&root, ".depth".into(), kind, Some(value));
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
            .push(&root, ".parts".into(), kind, value);
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
        let action = PropertyAction {
            root: Arc::new(HeldRoot),
            path: ".width".into(),
            kind: EditKind::Set,
            op: EditOp::Write(Box::new(0.0f32)),
            gesture: 0,
        };
        assert_eq!(action.label(), "Set Size.width");
        let through_option = PropertyAction {
            path: ".0.radius.0".into(),
            ..action
        };
        assert_eq!(through_option.label(), "Set Size.radius");
        let added = PropertyAction {
            path: ".depth".into(),
            kind: EditKind::Add,
            ..through_option
        };
        assert_eq!(added.label(), "Add Size.depth");
        let item = PropertyAction {
            path: ".0".into(),
            kind: EditKind::RemoveItem(1),
            ..added
        };
        assert_eq!(item.label(), "Remove Size item 2");
        let in_item = PropertyAction {
            path: ".0[1].shape".into(),
            kind: EditKind::Set,
            ..item
        };
        assert_eq!(in_item.label(), "Set Size.shape");
    }
}
