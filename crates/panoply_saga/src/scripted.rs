//! Aspects declared in SAGA, and the components that hold their fields on an entity.
//!
//! Each scripted aspect has two components, its config and its state, each registered at runtime
//! with an id of its own, so that change ticks and queries tell aspects apart. Every one of them
//! is laid out as one Rust type, [`ScriptedFields`], which holds the aspect's declaration and the
//! values of its config or state fields. Panoply reads them by id through [`ScriptedComponents`],
//! never through `ReflectComponent`, which would go by `ScriptedFields`' own id.
//!
//! The invariant every `unsafe` block here relies on: an id in [`ScriptedComponents`] was
//! registered by [`ScriptedComponents::register`] with `ScriptedFields`' layout and drop, so a
//! pointer to a component of that id points to a `ScriptedFields`.

use core::{alloc::Layout, mem::ManuallyDrop};
use std::{
    borrow::Cow,
    sync::{Arc, OnceLock, Weak},
};

use bevy::{
    ecs::{
        component::{ComponentCloneBehavior, ComponentDescriptor, ComponentId, StorageType},
        entity::{ComponentCloneCtx, Entity, SourceComponent},
        resource::Resource,
        world::{EntityWorldMut, Mut, World},
    },
    platform::collections::HashMap,
    ptr::{OwningPtr, Ptr},
    reflect::{PartialReflect, Reflect, TypeInfo, attributes::CustomAttributes},
};
use smol_str::SmolStr;

use crate::ScriptModule;

/// An aspect a `.saga` module declares (`aspect ToggleSwitch for Wall { ... }`).
#[derive(Debug)]
pub struct AspectDecl {
    /// The aspect key: the module's path from the asset root, then the aspect's name, as
    /// `scripts::toggle_switch::ToggleSwitch`.
    pub key: Arc<str>,
    /// The aspect's name as declared.
    pub name: SmolStr,
    /// The name formulas read it by from an entity (`self.toggle_switch`), from
    /// `#[accessor(...)]`. Without one, formulas cannot read it.
    pub accessor: Option<SmolStr>,
    /// The kinds of instance it attaches to, as written (`Wall`).
    pub kinds: Vec<SmolStr>,
    /// What its attributes but `accessor` give it, from the host's table
    /// ([`HostState::add_aspect_attribute`](crate::HostState::add_aspect_attribute)): its pane's
    /// icon.
    pub attributes: CustomAttributes,
    /// The keys of the aspects it requires, from `#[require(...)]`: where an instance's
    /// resolved aspects lack one, it attaches at its defaults (D121). A name the module declares
    /// an aspect by is that aspect's key; any other is a native aspect's.
    pub requires: Vec<Arc<str>>,
    /// Its fields, in declaration order.
    pub fields: Vec<AspectField>,
    /// The interactions it offers, in declaration order.
    pub interactions: Vec<InteractionDecl>,
    /// The module that declares it, whose functions its interactions run. Set once the module
    /// is shared ([`ScriptModule::share`]); weak, since the module holds the declaration.
    pub(crate) module: OnceLock<Weak<ScriptModule>>,
}

/// An interaction an aspect declares: its name, its constant fields' values, and each formula
/// or effect it sets as a function of its module that takes no arguments and runs on the entity
/// the aspect is on.
#[derive(Debug)]
pub struct InteractionDecl {
    pub name: SmolStr,
    /// Each constant field's name and value.
    pub constants: Vec<(SmolStr, Box<dyn Reflect>)>,
    /// Each formula or effect field's name and its function's index in the module.
    pub fields: Vec<(SmolStr, usize)>,
}

impl InteractionDecl {
    /// The index of the function that gives field `name`, if the interaction sets it.
    pub fn field(&self, name: &str) -> Option<usize> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, index)| *index)
    }

    /// The value of constant field `name`, if the interaction sets it and it is a `T`.
    pub fn constant<T: Reflect>(&self, name: &str) -> Option<&T> {
        self.constants
            .iter()
            .find(|(field, _)| field == name)
            .and_then(|(_, value)| value.downcast_ref())
    }

    /// Whether the interaction sets field `name`, of any kind.
    pub fn sets(&self, name: &str) -> bool {
        self.field(name).is_some() || self.constants.iter().any(|(field, _)| field == name)
    }
}

/// A field of a scripted aspect.
#[derive(Debug)]
pub struct AspectField {
    pub name: SmolStr,
    /// Whether it is state, which play changes, rather than config.
    pub is_state: bool,
    /// The field's Rust type, which the type registry knows.
    pub info: &'static TypeInfo,
    /// The value its declaration gives it, if any; without one, its type's default.
    pub default: Option<Box<dyn Reflect>>,
    /// What its attributes give it, from the host's table
    /// ([`HostState::add_field_attribute`](crate::HostState::add_field_attribute)): how the
    /// editor shows it, as a native field's `#[reflect(@...)]` does.
    pub attributes: CustomAttributes,
}

impl AspectDecl {
    /// Whether any field is state.
    pub fn has_state(&self) -> bool {
        self.fields.iter().any(|field| field.is_state)
    }

    /// The fields of one part: the state fields, or the config fields.
    pub fn fields_of(&self, state: bool) -> impl Iterator<Item = &AspectField> {
        self.fields
            .iter()
            .filter(move |field| field.is_state == state)
    }

    /// The field `name`.
    pub fn field(&self, name: &str) -> Option<&AspectField> {
        self.fields.iter().find(|field| field.name == name)
    }

    /// The module that declares the aspect, while something holds it.
    pub fn module(&self) -> Option<Arc<ScriptModule>> {
        self.module.get()?.upgrade()
    }
}

/// The values of a scripted aspect's config fields, or of its state fields: what each of its two
/// components holds.
#[derive(Debug)]
pub struct ScriptedFields {
    decl: Arc<AspectDecl>,
    state: bool,
    /// One value for each field of the part, in declaration order.
    values: Vec<Box<dyn Reflect>>,
}

impl ScriptedFields {
    /// The part `state` of `decl`, with `values` for its fields in order. `None` if there are not
    /// as many values as fields, or one is not of its field's type.
    pub fn new(decl: Arc<AspectDecl>, state: bool, values: Vec<Box<dyn Reflect>>) -> Option<Self> {
        let mut fields = decl.fields_of(state);
        let matches = values.iter().all(|value| {
            fields
                .next()
                .is_some_and(|field| value.as_any().type_id() == field.info.type_id())
        }) && fields.next().is_none();
        drop(fields);
        matches.then_some(Self {
            decl,
            state,
            values,
        })
    }

    /// The aspect these are fields of.
    pub fn decl(&self) -> &Arc<AspectDecl> {
        &self.decl
    }

    /// Whether these are the state fields, rather than the config fields.
    pub fn is_state(&self) -> bool {
        self.state
    }

    /// Each field with its value.
    pub fn iter(&self) -> impl Iterator<Item = (&AspectField, &dyn Reflect)> {
        self.decl
            .fields_of(self.state)
            .zip(self.values.iter().map(|value| value.as_ref()))
    }

    /// The value of field `name`.
    pub fn get(&self, name: &str) -> Option<&dyn Reflect> {
        self.iter()
            .find(|(field, _)| field.name == name)
            .map(|(_, value)| value)
    }

    /// The value of field `name`, to change. Its type stays the field's: a value applied to it
    /// goes through `PartialReflect::try_apply`.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        let index = self
            .decl
            .fields_of(self.state)
            .position(|f| f.name == name)?;
        Some(self.values[index].as_partial_reflect_mut())
    }

    /// A copy, if every value can be cloned through reflection.
    pub fn try_clone(&self) -> Option<Self> {
        let values = self
            .values
            .iter()
            .map(|value| value.reflect_clone().ok())
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            decl: self.decl.clone(),
            state: self.state,
            values,
        })
    }
}

/// The ids of a scripted aspect's two components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptedIds {
    pub config: ComponentId,
    pub state: ComponentId,
}

impl ScriptedIds {
    /// The id of one part's component.
    pub fn of(&self, state: bool) -> ComponentId {
        if state { self.state } else { self.config }
    }
}

/// The component ids of each scripted aspect this world has attached, by aspect key.
#[derive(Resource, Default, Debug)]
pub struct ScriptedComponents(HashMap<Arc<str>, ScriptedIds>);

impl ScriptedComponents {
    /// The ids of the aspect `key`, if this world has registered it.
    pub fn ids(&self, key: &str) -> Option<ScriptedIds> {
        self.0.get(key).copied()
    }

    /// The ids of the aspect `key`, registering its components the first time.
    pub fn register(world: &mut World, key: &Arc<str>) -> ScriptedIds {
        if let Some(ids) = world
            .get_resource::<ScriptedComponents>()
            .and_then(|components| components.ids(key))
        {
            return ids;
        }
        let mut descriptor = |part: &str| {
            // SAFETY: the layout and drop are `ScriptedFields`', and every pointer inserted with
            // these ids points to one (`insert`).
            let descriptor = unsafe {
                ComponentDescriptor::new_with_layout(
                    Cow::Owned(format!("{key} ({part})")),
                    StorageType::Table,
                    Layout::new::<ScriptedFields>(),
                    Some(drop_scripted_fields),
                    true,
                    false,
                    ComponentCloneBehavior::Custom(clone_scripted_fields),
                    None,
                )
            };
            world.register_component_with_descriptor(descriptor)
        };
        let ids = ScriptedIds {
            config: descriptor("config"),
            state: descriptor("state"),
        };
        world
            .get_resource_or_init::<ScriptedComponents>()
            .0
            .insert(key.clone(), ids);
        ids
    }
}

/// Drops a `ScriptedFields` in place.
///
/// # Safety
///
/// `ptr` points to a `ScriptedFields`, as every component registered by
/// [`ScriptedComponents::register`] is.
unsafe fn drop_scripted_fields(ptr: OwningPtr<'_>) {
    // SAFETY: upheld by the caller.
    unsafe { ptr.drop_as::<ScriptedFields>() }
}

/// Clones a scripted aspect's component onto the clone's target. A value that cannot be cloned
/// through reflection leaves the target without the component.
fn clone_scripted_fields(source: &SourceComponent, ctx: &mut ComponentCloneCtx) {
    // SAFETY: the clone handler is registered only with ids of `ScriptedFields` components.
    let fields = unsafe { source.ptr().deref::<ScriptedFields>() };
    let Some(copy) = fields.try_clone() else {
        return;
    };
    // The bytes are moved into the target, which owns them from here.
    let copy = ManuallyDrop::new(copy);
    // SAFETY: `copy` is a `ScriptedFields`, the type the target's id stands for, and it is not
    // dropped here, so the target owns its values alone.
    unsafe { ctx.write_target_component_ptr(Ptr::from(&*copy)) };
}

/// Inserts `fields` on `entity`, as its aspect's config or state component, replacing any.
pub fn insert(entity: &mut EntityWorldMut, fields: ScriptedFields) {
    let key = fields.decl.key.clone();
    let ids = entity.world_scope(|world| ScriptedComponents::register(world, &key));
    let id = ids.of(fields.state);
    OwningPtr::make(fields, |ptr| {
        // SAFETY: `id` is from this entity's world, registered for `ScriptedFields`, and `ptr`
        // points to one.
        unsafe {
            entity.insert_by_id(id, ptr);
        }
    });
}

/// Removes the config or state component of the aspect `key` from `entity`.
pub fn remove(entity: &mut EntityWorldMut, key: &str, state: bool) {
    let ids = entity.world_scope(|world| {
        world
            .get_resource::<ScriptedComponents>()
            .and_then(|components| components.ids(key))
    });
    if let Some(ids) = ids {
        entity.remove_by_id(ids.of(state));
    }
}

/// Whether `entity` has the config or state component of the aspect `key`.
pub fn contains(entity: &EntityWorldMut, key: &str, state: bool) -> bool {
    entity
        .world()
        .get_resource::<ScriptedComponents>()
        .and_then(|components| components.ids(key))
        .is_some_and(|ids| entity.contains_id(ids.of(state)))
}

/// The config or state component of the aspect `key` on `entity`, and its id, which is `None`
/// if the world has never registered the aspect.
pub fn get<'w>(
    world: &'w World,
    entity: Entity,
    key: &str,
    state: bool,
) -> (Option<ComponentId>, Option<&'w ScriptedFields>) {
    let Some(id) = world
        .get_resource::<ScriptedComponents>()
        .and_then(|components| components.ids(key))
        .map(|ids| ids.of(state))
    else {
        return (None, None);
    };
    // SAFETY: `id` is registered for `ScriptedFields`.
    let fields = world
        .get_by_id(entity, id)
        .map(|ptr| unsafe { ptr.deref::<ScriptedFields>() });
    (Some(id), fields)
}

/// The config or state component of the aspect `key` on `entity`, to change.
pub fn get_mut<'w>(
    world: &'w mut World,
    entity: Entity,
    key: &str,
    state: bool,
) -> Option<Mut<'w, ScriptedFields>> {
    let id = world
        .get_resource::<ScriptedComponents>()?
        .ids(key)?
        .of(state);
    let component = world.get_mut_by_id(entity, id)?;
    // SAFETY: `id` is registered for `ScriptedFields`.
    Some(component.map_unchanged(|ptr| unsafe { ptr.deref_mut::<ScriptedFields>() }))
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::entity::EntityCloner, reflect::Typed};

    use super::*;

    fn decl() -> Arc<AspectDecl> {
        Arc::new(AspectDecl {
            key: "scripts::test::Lamp".into(),
            name: "Lamp".into(),
            accessor: Some("lamp".into()),
            kinds: vec!["Fixture".into()],
            attributes: CustomAttributes::default(),
            requires: Vec::new(),
            fields: vec![
                AspectField {
                    name: "color".into(),
                    is_state: false,
                    info: String::type_info(),
                    default: None,
                    attributes: CustomAttributes::default(),
                },
                AspectField {
                    name: "lit".into(),
                    is_state: true,
                    info: bool::type_info(),
                    default: None,
                    attributes: CustomAttributes::default(),
                },
            ],
            interactions: Vec::new(),
            module: OnceLock::new(),
        })
    }

    fn lit(world: &World, entity: Entity) -> Option<bool> {
        let (_, fields) = get(world, entity, "scripts::test::Lamp", true);
        fields?.get("lit")?.downcast_ref::<bool>().copied()
    }

    #[test]
    fn fields_must_match_their_part() {
        let decl = decl();
        assert!(ScriptedFields::new(decl.clone(), true, vec![Box::new(true)]).is_some());
        assert!(ScriptedFields::new(decl.clone(), true, vec![Box::new(1.0f32)]).is_none());
        assert!(ScriptedFields::new(decl.clone(), true, Vec::new()).is_none());
        assert!(ScriptedFields::new(decl, false, vec![Box::new(true)]).is_none());
    }

    #[test]
    fn parts_are_components_of_their_own() {
        let mut world = World::new();
        let decl = decl();
        let mut entity = world.spawn_empty();
        insert(
            &mut entity,
            ScriptedFields::new(decl.clone(), false, vec![Box::new("red".to_string())]).unwrap(),
        );
        insert(
            &mut entity,
            ScriptedFields::new(decl.clone(), true, vec![Box::new(false)]).unwrap(),
        );
        assert!(contains(&entity, &decl.key, false));
        assert!(contains(&entity, &decl.key, true));
        let entity = entity.id();
        assert_eq!(lit(&world, entity), Some(false));

        *get_mut(&mut world, entity, &decl.key, true)
            .unwrap()
            .get_mut("lit")
            .unwrap()
            .try_downcast_mut::<bool>()
            .unwrap() = true;
        assert_eq!(lit(&world, entity), Some(true));

        let ids = world
            .resource::<ScriptedComponents>()
            .ids(&decl.key)
            .unwrap();
        assert_ne!(ids.config, ids.state);

        remove(&mut world.entity_mut(entity), &decl.key, false);
        let entity_ref = world.entity_mut(entity);
        assert!(!contains(&entity_ref, &decl.key, false));
        assert!(contains(&entity_ref, &decl.key, true));
        // Despawning drops the values.
        world.despawn(entity);
    }

    #[test]
    fn cloning_an_entity_copies_its_scripted_components() {
        let mut world = World::new();
        let decl = decl();
        let mut entity = world.spawn_empty();
        insert(
            &mut entity,
            ScriptedFields::new(decl.clone(), true, vec![Box::new(true)]).unwrap(),
        );
        let source = entity.id();
        let target = world.spawn_empty().id();
        EntityCloner::build_opt_out(&mut world).clone_entity(source, target);
        assert_eq!(lit(&world, target), Some(true));

        // The copy is its own: changing it leaves the source as it was.
        *get_mut(&mut world, target, &decl.key, true)
            .unwrap()
            .get_mut("lit")
            .unwrap()
            .try_downcast_mut::<bool>()
            .unwrap() = false;
        assert_eq!(lit(&world, source), Some(true));
        assert_eq!(lit(&world, target), Some(false));
    }
}
