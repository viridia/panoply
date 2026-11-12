//! What the property editor edits, and how its edits are made. A [`PropertyRoot`] is where a value
//! lives, named by a stable key (D49). The panel never writes a value: each widget emits a
//! [`PropertyEdit`], a root, a path within it and the new value, and [`apply_property_edits`] makes
//! the edits after the panel has run and records each in the undo history. Then
//! [`InspectedValues`] takes a fresh copy of each value the panel shows, so the next frame's panel
//! draws what the edits left. Only the panel's walk and the roots touch reflection.

use std::{any::Any, borrow::Cow, sync::Arc};

use bevy::{asset::AssetPath, prelude::*, reflect::GetPath};
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

    /// Writes `value` at `path` within the value, in [`GetPath`] syntax, and returns the value it
    /// replaced.
    fn set(
        &self,
        world: &mut World,
        path: &str,
        value: &dyn PartialReflect,
    ) -> Result<Box<dyn PartialReflect>, BevyError>;
}

/// What an edit does, as the undo history names it. Each is a value written at a path: adding an
/// optional field writes `Some` at its `Option`, and removing it writes `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Set,
    Add,
    Remove,
}

impl EditKind {
    fn verb(self) -> &'static str {
        match self {
            EditKind::Set => "Set",
            EditKind::Add => "Add",
            EditKind::Remove => "Remove",
        }
    }
}

/// An edit a widget asks for: `value` written at `path` within `root`'s value.
pub struct PropertyEdit {
    pub root: Arc<dyn PropertyRoot>,
    pub path: String,
    pub kind: EditKind,
    pub value: Box<dyn PartialReflect>,
}

/// The edits asked for this frame, made by [`apply_property_edits`].
#[derive(Resource, Default)]
pub struct PropertyEdits(pub Vec<PropertyEdit>);

impl PropertyEdits {
    /// Asks for `value` to be written at `path` within `root`'s value.
    pub fn set(&mut self, root: &Arc<dyn PropertyRoot>, path: String, value: impl PartialReflect) {
        self.push(root, path, EditKind::Set, Box::new(value));
    }

    /// Asks for `value` to be written at `path` within `root`'s value, as a `kind` of edit.
    pub fn push(
        &mut self,
        root: &Arc<dyn PropertyRoot>,
        path: String,
        kind: EditKind,
        value: Box<dyn PartialReflect>,
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

/// Writes a value at a path within a root, and reverses to the value it replaced.
struct SetProperty {
    root: Arc<dyn PropertyRoot>,
    path: String,
    kind: EditKind,
    value: Box<dyn PartialReflect>,
    gesture: u32,
}

impl UndoAction for SetProperty {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        match self.root.set(world, &self.path, self.value.as_ref()) {
            Ok(replaced) => Box::new(SetProperty {
                value: replaced,
                ..*self
            }),
            Err(error) => {
                error!("Could not set {}: {error}", self.label());
                self
            }
        }
    }

    /// "Set LightSource.radius", or "Add" or "Remove": the path's field names, leaving out the
    /// steps into an `Option` or a newtype, which the panel does not show either.
    fn label(&self) -> Cow<'static, str> {
        let mut label = format!("{} {}", self.kind.verb(), self.root.caption());
        for name in self.path.split('.') {
            if !name.is_empty() && !name.bytes().all(|byte| byte.is_ascii_digit()) {
                label.push('.');
                label.push_str(name);
            }
        }
        label.into()
    }

    fn coalesces_into(&self, earlier: &dyn UndoAction) -> bool {
        (earlier as &dyn Any)
            .downcast_ref::<SetProperty>()
            .is_some_and(|earlier| {
                earlier.gesture == self.gesture
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
        match root.set(world, &path, value.as_ref()) {
            Ok(replaced) => world.resource_mut::<UndoStack>().push(
                mode,
                SetProperty {
                    root,
                    path,
                    kind,
                    value: replaced,
                    gesture,
                },
            ),
            Err(error) => error!("Could not set {}{path}: {error}", root.caption()),
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

    fn set(
        &self,
        world: &mut World,
        path: &str,
        value: &dyn PartialReflect,
    ) -> Result<Box<dyn PartialReflect>, BevyError> {
        let id = self
            .exemplar_id(world)
            .ok_or_else(|| format!("{} is not loaded", self.exemplar))?;
        let catalog = world
            .resource::<AssetServer>()
            .get_path_id(self.exemplar.without_label())
            .ok_or_else(|| format!("{} has no catalog", self.exemplar))?;
        let replaced = world.resource_scope(|_, mut exemplars: Mut<Assets<Exemplar>>| {
            Exemplar::edit(&mut exemplars, id, |data| {
                let aspect = data
                    .aspects
                    .iter_mut()
                    .find(|aspect| aspect.reflect_short_type_path() == self.aspect)
                    .ok_or_else(|| format!("{} has no {}", self.exemplar, self.aspect))?;
                let field = aspect
                    .as_reflect_mut()
                    .reflect_path_mut(path)
                    .map_err(|error| error.to_string())?;
                let replaced = field.reflect_clone()?.into_partial_reflect();
                field.try_apply(value)?;
                Ok(replaced)
            })
        })?;
        world.resource_mut::<UnsavedAssets>().mark(catalog);
        Ok(replaced)
    }
}

#[cfg(test)]
mod tests {
    use bevy::reflect::TypeRegistry;

    use super::*;
    use crate::properties::option_value;

    #[derive(Reflect, Clone, Default, Debug, PartialEq)]
    struct Size {
        width: f32,
        height: f32,
        depth: Option<f32>,
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

        fn set(
            &self,
            world: &mut World,
            path: &str,
            value: &dyn PartialReflect,
        ) -> Result<Box<dyn PartialReflect>, BevyError> {
            let mut held = world.resource_mut::<Held>();
            let field = held.0.reflect_path_mut(path).map_err(|e| e.to_string())?;
            let replaced = field.reflect_clone()?.into_partial_reflect();
            field.try_apply(value)?;
            Ok(replaced)
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
            .push(&root, ".depth".into(), kind, value);
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

    #[test]
    fn the_label_names_the_field() {
        let action = SetProperty {
            root: Arc::new(HeldRoot),
            path: ".width".into(),
            kind: EditKind::Set,
            value: Box::new(0.0f32),
            gesture: 0,
        };
        assert_eq!(action.label(), "Set Size.width");
        let through_option = SetProperty {
            path: ".0.radius.0".into(),
            ..action
        };
        assert_eq!(through_option.label(), "Set Size.radius");
        let added = SetProperty {
            path: ".depth".into(),
            kind: EditKind::Add,
            ..through_option
        };
        assert_eq!(added.label(), "Add Size.depth");
    }
}
