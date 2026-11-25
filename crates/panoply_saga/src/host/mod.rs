//! What the host offers scripts: globals, functions, and types with their members.
//!
//! Registration is the host's own Rust, run at startup, so a conflicting name panics there.

pub(crate) mod layout;
mod string;

use std::{any::TypeId, sync::Arc};

use bevy::{
    ecs::{component::Component, entity::Entity, world::World},
    platform::collections::HashMap,
    reflect::{PartialReflect, Reflect, Typed},
};
use smol_str::SmolStr;

use crate::{
    ScriptVm,
    reactive::{NativeMemo, call_memo},
    sema::{
        decl::{Decl, DeclKind, DeclTable, DeclVisibility, FunctionParam},
        types::{ExprType, FunctionType, HostParam},
    },
    syntax::location::TokenLocation,
    tracking::TrackingScope,
    vm::{InvocationContext, StackValue, VMError},
};
use layout::Layout;

/// A native function, method or property: it reads its arguments (a method's or property's
/// first is the instance) with [`InvocationContext::arg`] and writes its result with
/// [`InvocationContext::ret`].
pub type HostFunction = fn(&mut InvocationContext) -> Result<(), VMError>;
/// A global property's value. It takes the VM mutably to track what it reads.
pub type GlobalPropertyAccessor = fn(&mut ScriptVm) -> Result<StackValue, VMError>;

/// A component scripts read from an entity, a range of its slots at a time.
#[derive(Debug)]
pub(crate) struct ComponentRead {
    /// The component on an entity, its read tracked; an error if the entity lacks it.
    pub(crate) read: for<'w> fn(
        &'w World,
        Entity,
        &mut TrackingScope,
    ) -> Result<&'w dyn PartialReflect, VMError>,
    pub(crate) layout: Layout,
}

/// Symbol table containing the methods and fields of a composite type such as a struct.
#[derive(Default, Debug)]
pub struct HostType {
    /// Lookup scope for object properties and methods.
    pub(crate) decls: DeclTable,
}

impl HostType {
    pub fn get(&self, name: &str) -> Option<&Decl> {
        self.decls.get(name)
    }
}

fn host_decl(typ: ExprType, kind: DeclKind) -> Decl {
    Decl {
        location: TokenLocation::default(),
        visibility: DeclVisibility::Public,
        typ,
        kind,
    }
}

fn function_type(params: Vec<HostParam>, ret: ExprType) -> ExprType {
    ExprType::Function(Arc::new(FunctionType {
        params: params
            .into_iter()
            .enumerate()
            .map(|(index, p)| FunctionParam {
                location: TokenLocation::default(),
                name: p.name,
                typ: p.ty,
                index,
            })
            .collect(),
        ret,
    }))
}

pub struct HostTypeBuilder<'host> {
    decls: &'host mut DeclTable,
    members: &'host mut Vec<HostMember>,
}

impl HostTypeBuilder<'_> {
    fn add(&mut self, name: SmolStr, member: HostMember, typ: ExprType, function: bool) {
        assert!(
            !self.decls.contains_key(&name),
            "Object member {name} is already defined."
        );
        let index = self.members.len();
        self.members.push(member);
        let kind = if function {
            DeclKind::Function { index }
        } else {
            DeclKind::Global {
                is_const: true,
                index,
            }
        };
        self.decls.insert(name, host_decl(typ, kind));
    }

    /// Add a property to this type, a native called with the instance as its one argument.
    pub fn add_property(
        &mut self,
        name: impl Into<SmolStr>,
        accessor: HostFunction,
        typ: ExprType,
    ) -> &mut Self {
        self.add(
            name.into(),
            HostMember::InstanceProperty(accessor),
            typ,
            false,
        );
        self
    }

    /// Add a native method to this type, called with the instance as its first argument.
    pub fn add_method(
        &mut self,
        name: impl Into<SmolStr>,
        method: HostFunction,
        params: Vec<HostParam>,
        return_type: ExprType,
    ) -> &mut Self {
        let typ = function_type(params, return_type);
        self.add(name.into(), HostMember::InstanceMethod(method), typ, true);
        self
    }

    /// Add a native function to this type, called as `Type::name(...)`.
    pub fn add_static_method(
        &mut self,
        name: impl Into<SmolStr>,
        method: HostFunction,
        params: Vec<HostParam>,
        return_type: ExprType,
    ) -> &mut Self {
        let typ = function_type(params, return_type);
        self.add(name.into(), HostMember::Function(method), typ, true);
        self
    }
}

#[derive(Debug)]
pub(crate) enum HostMember {
    /// A constant value.
    Const(StackValue),

    /// A dynamic property, such as `time`.
    GlobalProperty(GlobalPropertyAccessor),

    /// A callable function which does not require a `self` argument.
    Function(HostFunction),

    /// A property of a type
    InstanceProperty(HostFunction),

    /// A method of a type
    InstanceMethod(HostFunction),

    /// A component of an entity, read by [`HostState::add_component`]'s accessor.
    Component(ComponentRead),
}

/// Contains global definitions and method registrations for the compiler and VM.
#[derive(Default, Debug)]
pub struct HostState {
    /// Lookup scope for global variables, functions, and properties. This contains all
    /// host symbols that can be looked up without a scope.
    pub(crate) decls: DeclTable,

    /// Table of global variables, properties, and functions.
    pub(crate) members: Vec<HostMember>,

    /// List of known native types
    pub(crate) types: HashMap<TypeId, HostType>,

    /// The member index of `self`, if the host has it.
    pub(crate) self_index: Option<usize>,

    /// The component each component accessor reads, by member index.
    pub(crate) component_members: HashMap<usize, TypeId>,

    /// The fields an aspect's interactions may set: the interaction type.
    pub(crate) interaction_fields: Vec<InteractionField>,
}

/// What a field of an interaction holds.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    /// A literal of this type, fixed in the declaration: a caption. An interaction whose
    /// caption changes is two interactions, each with its own `when`.
    Constant(ExprType),
    /// A read-only formula returning a value of this type: a `when`.
    Formula(ExprType),
    /// May write its aspect's state, through the effect buffer; it returns nothing.
    Effect,
}

/// A field of the interaction type, registered with [`HostState::add_interaction_field`].
#[derive(Debug, Clone)]
pub struct InteractionField {
    pub name: SmolStr,
    pub kind: FieldKind,
    /// Whether every interaction must set it.
    pub required: bool,
}

impl InteractionField {
    /// The type of its value.
    pub fn typ(&self) -> ExprType {
        match &self.kind {
            FieldKind::Constant(typ) | FieldKind::Formula(typ) => typ.clone(),
            FieldKind::Effect => ExprType::Void,
        }
    }
}

impl HostState {
    /// A host with the primitive types, `Entity` and `String`, `String`'s methods, and `self`,
    /// the entity a script runs on.
    pub fn new() -> Self {
        let mut this = Self::default();
        this.add_type_alias("i32", ExprType::I32);
        this.add_type_alias("i64", ExprType::I64);
        this.add_type_alias("f32", ExprType::F32);
        this.add_type_alias("f64", ExprType::F64);
        this.add_type_alias("bool", ExprType::Boolean);
        this.add_host_type::<Entity>("Entity");
        this.add_host_type::<String>("String");
        string::init_string_methods(&mut this);
        this.self_index = Some(this.add_global_prop("self", read_self, ExprType::Entity));
        this
    }

    /// Adds `name`, a property of every entity that reads its component `C`, as `self.name`. It
    /// tracks the read; on an entity without `C`, the evaluation fails. A formula's reads of it
    /// through `self` are listed in [`ScriptModule::self_components`](crate::ScriptModule).
    pub fn add_component<C: Component + Reflect + Typed>(&mut self, name: impl Into<SmolStr>) {
        let typ = ExprType::from_type_info(C::type_info());
        let member = HostMember::Component(ComponentRead {
            read: read_component::<C>,
            layout: Layout::of(C::type_info()),
        });
        self.get_host_type_mut::<Entity>()
            .expect("HostState::new adds Entity")
            .add(name.into(), member, typ, false);
        self.component_members
            .insert(self.members.len() - 1, TypeId::of::<C>());
    }

    /// Adds `name` to the fields an aspect's interactions may set. An interaction that sets a
    /// field not added, or leaves out a required one, fails to compile.
    pub fn add_interaction_field(
        &mut self,
        name: impl Into<SmolStr>,
        kind: FieldKind,
        required: bool,
    ) {
        let name = name.into();
        assert!(
            self.interaction_field(&name).is_none(),
            "Interaction field {name} is already defined."
        );
        self.interaction_fields.push(InteractionField {
            name,
            kind,
            required,
        });
    }

    /// The interaction field `name`, if the host has added it.
    pub fn interaction_field(&self, name: &str) -> Option<&InteractionField> {
        self.interaction_fields.iter().find(|f| f.name == name)
    }

    /// Whether member `index` is a component accessor.
    pub(crate) fn is_component(&self, index: usize) -> bool {
        matches!(self.members.get(index), Some(HostMember::Component(_)))
    }

    fn add_global(&mut self, name: SmolStr, decl: Decl) {
        assert!(
            !self.decls.contains_key(&name),
            "Host symbol {name} is already defined."
        );
        self.decls.insert(name, decl);
    }

    pub fn add_type_alias(&mut self, name: impl Into<SmolStr>, typ: ExprType) {
        self.add_global(name.into(), host_decl(typ, DeclKind::TypeAlias));
    }

    /// Adds a constant.
    ///
    /// # Panics
    ///
    /// If the value is not a primitive or an entity.
    pub fn add_global_const(&mut self, name: impl Into<SmolStr>, value: StackValue) -> usize {
        let typ = match value {
            StackValue::Bool(_) => ExprType::Boolean,
            StackValue::I32(_) => ExprType::I32,
            StackValue::I64(_) => ExprType::I64,
            StackValue::F32(_) => ExprType::F32,
            StackValue::F64(_) => ExprType::F64,
            StackValue::Entity(_) => ExprType::Entity,
            _ => panic!("Unsupported constant type"),
        };
        let index = self.members.len();
        self.add_global(
            name.into(),
            host_decl(
                typ,
                DeclKind::Global {
                    is_const: true,
                    index,
                },
            ),
        );
        self.members.push(HostMember::Const(value));
        index
    }

    pub fn add_global_prop(
        &mut self,
        name: impl Into<SmolStr>,
        accessor: GlobalPropertyAccessor,
        typ: ExprType,
    ) -> usize {
        let index = self.members.len();
        self.add_global(
            name.into(),
            host_decl(
                typ,
                DeclKind::Global {
                    is_const: true,
                    index,
                },
            ),
        );
        self.members.push(HostMember::GlobalProperty(accessor));
        index
    }

    /// Adds a native function, called as `name(...)`.
    pub fn add_function(
        &mut self,
        name: impl Into<SmolStr>,
        function: HostFunction,
        params: Vec<HostParam>,
        return_type: ExprType,
    ) -> usize {
        let index = self.members.len();
        self.add_global(
            name.into(),
            host_decl(
                function_type(params, return_type),
                DeclKind::Function { index },
            ),
        );
        self.members.push(HostMember::Function(function));
        index
    }

    /// Adds a native memo, called as `name(...)`: within a cell it is kept between evaluations,
    /// one per call site (see the crate's Terms). Its parameters are primitives or entities.
    pub fn add_memo<M: NativeMemo>(
        &mut self,
        name: impl Into<SmolStr>,
        params: Vec<HostParam>,
        return_type: ExprType,
    ) -> usize {
        self.add_function(name, call_memo::<M>, params, return_type)
    }

    pub fn add_host_type<T: Typed>(&mut self, name: impl Into<SmolStr>) -> HostTypeBuilder<'_> {
        let id = TypeId::of::<T>();
        self.add_global(
            name.into(),
            host_decl(
                ExprType::from_type_info(T::type_info()),
                DeclKind::NativeType { id },
            ),
        );
        let host_type = self.types.entry(id).or_default();
        HostTypeBuilder {
            decls: &mut host_type.decls,
            members: &mut self.members,
        }
    }

    /// Returns the `HostType` table for a native type.
    pub fn get_host_type<T: Typed>(&self) -> Option<&HostType> {
        self.types.get(&TypeId::of::<T>())
    }

    /// Returns a builder for a native type already added.
    pub fn get_host_type_mut<T: Typed>(&mut self) -> Option<HostTypeBuilder<'_>> {
        self.types
            .get_mut(&TypeId::of::<T>())
            .map(|host_type| HostTypeBuilder {
                decls: &mut host_type.decls,
                members: &mut self.members,
            })
    }

    /// Returns the `HostType` table for a native type.
    pub fn get_host_type_by_id(&self, id: TypeId) -> Option<&HostType> {
        self.types.get(&id)
    }
}

/// `self`: the entity the script runs on.
fn read_self(vm: &mut ScriptVm) -> Result<StackValue, VMError> {
    Ok(StackValue::Entity(vm.owner))
}

/// The accessor [`HostState::add_component`] registers: `entity`'s `C`.
fn read_component<'w, C: Component + Reflect>(
    world: &'w World,
    entity: Entity,
    tracking: &mut TrackingScope,
) -> Result<&'w dyn PartialReflect, VMError> {
    let component = world
        .get_entity(entity)
        .ok()
        .and_then(|entity| entity.get::<C>());
    tracking.track_component::<C>(entity, world, component.is_some());
    component
        .map(|c| c.as_partial_reflect())
        .ok_or(VMError::MissingComponent(core::any::type_name::<C>()))
}
