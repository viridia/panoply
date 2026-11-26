//! Aspects declared in SAGA ([`AspectDecl`]), as aspects of an exemplar or instance.

use std::sync::Arc;

use bevy::{
    log::warn,
    prelude::*,
    reflect::{
        serde::{TypedReflectDeserializer, TypedReflectSerializer},
        TypeRegistry,
    },
};
use panoply_saga::{
    scripted::{self, AspectDecl, AspectField, ScriptedFields},
    ScriptModule,
};
use serde::de::DeserializeSeed;
use serde_json::{Map, Value};

use crate::{Aspect, InstanceKind};

/// A scripted aspect with its authored values: its config, and the values its state starts
/// from (D74). Attaching inserts the config as one component and, where the entity has none
/// yet, the state as another; detaching removes the config and leaves the state.
///
/// It is reflected as an opaque value, and registers no `ReflectAspect` or `ReflectDefault`: the
/// files and the editor reach it by its key, through its declaration.
#[derive(Reflect, Clone)]
#[reflect(opaque, Clone, Debug)]
pub struct ScriptedAspect {
    config: Arc<ScriptedFields>,
    state: Arc<ScriptedFields>,
    /// The module that declares it, which runs its interactions; held here, since its
    /// declaration holds it only weakly. `None` for a declaration no shared module holds.
    module: Option<Arc<ScriptModule>>,
}

/// Its name and fields, as a struct prints: `ToggleSwitch { caption: "Use", on: false }`.
impl std::fmt::Debug for ScriptedAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct(&self.decl().name);
        for (field, value) in self.fields() {
            out.field(&field.name, &value);
        }
        out.finish()
    }
}

impl ScriptedAspect {
    /// The aspect `decl` as `value`, an object of its fields by name, holds it. A field `value`
    /// lacks takes its default; one it holds that `decl` does not declare is skipped, with a
    /// warning naming `path`.
    pub fn from_json(
        decl: &Arc<AspectDecl>,
        value: &Value,
        registry: &TypeRegistry,
        path: &str,
    ) -> Result<Self, String> {
        if let Value::Object(object) = value {
            for name in object.keys() {
                if decl.field(name).is_none() {
                    warn!("{path}: {} has no field {name}; skipping it", decl.key);
                }
            }
        }
        Ok(Self {
            config: Arc::new(part_from_json(decl, false, value, registry, path)?),
            state: Arc::new(part_from_json(decl, true, value, registry, path)?),
            module: decl.module(),
        })
    }

    /// The aspect `decl` with every field at its default.
    pub fn new(decl: &Arc<AspectDecl>, registry: &TypeRegistry) -> Result<Self, String> {
        Self::from_json(decl, &Value::Null, registry, &decl.key)
    }

    /// The aspect's declaration.
    pub fn decl(&self) -> &Arc<AspectDecl> {
        self.config.decl()
    }

    /// The module that declares it, which runs its interactions.
    pub fn module(&self) -> Option<&Arc<ScriptModule>> {
        self.module.as_ref()
    }

    /// The values of its config fields.
    pub fn config(&self) -> &ScriptedFields {
        &self.config
    }

    /// The values its state starts from.
    pub fn initial_state(&self) -> &ScriptedFields {
        &self.state
    }

    /// Each field with its value, config and state alike, in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = (&AspectField, &dyn Reflect)> {
        self.decl().fields.iter().filter_map(|field| {
            let value = self.part(field.is_state).get(&field.name)?;
            Some((field, value))
        })
    }

    /// The value of field `name`, config or state, to change, as
    /// [`ScriptedFields::get_mut`] lends it. The part it is in is copied first if it is shared,
    /// with the other aspects this one was cloned from or into; `None` also if it cannot be.
    pub fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        let state = self.decl().field(name)?.is_state;
        let part = if state {
            &mut self.state
        } else {
            &mut self.config
        };
        if Arc::get_mut(part).is_none() {
            *part = Arc::new(part.try_clone()?);
        }
        Arc::get_mut(part)?.get_mut(name)
    }

    fn part(&self, state: bool) -> &ScriptedFields {
        if state {
            &self.state
        } else {
            &self.config
        }
    }
}

/// `field`'s value as `json` holds it, or its default where `json` lacks it or holds one not of
/// its type, which is warned of.
fn field_value(
    field: &AspectField,
    json: Option<&Value>,
    registry: &TypeRegistry,
    context: &str,
) -> Result<Box<dyn Reflect>, String> {
    let registration = registry
        .get(field.info.type_id())
        .ok_or_else(|| format!("{context}: {} is not registered", field.info.type_path()))?;
    let mut value = match &field.default {
        Some(default) => default
            .reflect_clone()
            .map_err(|err| format!("{context}: {err}"))?,
        None => registration
            .data::<ReflectDefault>()
            .ok_or_else(|| {
                format!(
                    "{context}: {} needs a default, since {} has none",
                    field.name,
                    field.info.type_path()
                )
            })?
            .default(),
    };
    if let Some(json) = json {
        match TypedReflectDeserializer::new(registration, registry).deserialize(json) {
            Ok(read) => {
                if let Err(err) = value.try_apply(&*read) {
                    warn!("{context}: {}: {err}; it keeps its default", field.name);
                }
            }
            Err(err) => warn!("{context}: {}: {err}; it keeps its default", field.name),
        }
    }
    Ok(value)
}

/// The config or state part of the aspect `decl`, from `value`, an object of fields by name,
/// as an aspect's entry in a file or a save holds it. Fields `value` lacks take their defaults,
/// and those it holds that the part does not declare are ignored, so that a file written
/// before the module changed still loads.
pub fn part_from_json(
    decl: &Arc<AspectDecl>,
    state: bool,
    value: &Value,
    registry: &TypeRegistry,
    path: &str,
) -> Result<ScriptedFields, String> {
    let object = match value {
        Value::Object(object) => Some(object),
        Value::Null => None,
        _ => return Err(format!("{path}: {} is not an object", decl.key)),
    };
    let context = format!("{path}: {}", decl.key);
    let values = decl
        .fields_of(state)
        .map(|field| {
            let json = object.and_then(|object| object.get(field.name.as_str()));
            field_value(field, json, registry, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;
    ScriptedFields::new(decl.clone(), state, values)
        .ok_or_else(|| format!("{context}: a field's default is not of its type"))
}

/// One field's value as JSON.
fn field_json(value: &dyn Reflect, registry: &TypeRegistry) -> Result<Value, BevyError> {
    let serializer = TypedReflectSerializer::new(value.as_partial_reflect(), registry);
    // Through text, as `save::reflect_json`, so an `f32` writes as its shortest decimal.
    Ok(serde_json::from_str(&serde_json::to_string(&serializer)?)?)
}

/// `fields`, one part of an aspect, as an object of its values by field name: what a save
/// keeps of its state.
pub fn part_to_json(fields: &ScriptedFields, registry: &TypeRegistry) -> Result<Value, BevyError> {
    let mut object = Map::new();
    for (field, value) in fields.iter() {
        object.insert(field.name.to_string(), field_json(value, registry)?);
    }
    Ok(object.into())
}

impl Aspect for ScriptedAspect {
    fn key(&self) -> &str {
        &self.decl().key
    }

    /// Its kinds are written as files write an exemplar's `kind`.
    fn can_attach(&self, kind: InstanceKind) -> bool {
        self.decl()
            .kinds
            .iter()
            .any(|name| name.parse() == Ok(kind))
    }

    fn attach(&self, entity: &mut EntityWorldMut) {
        let decl = self.decl().clone();
        if decl.fields_of(false).next().is_some() {
            match self.config.try_clone() {
                Some(config) => scripted::insert(entity, config),
                None => error!("{}: a config value cannot be cloned", decl.key),
            }
        }
        if decl.has_state() && !scripted::contains(entity, &decl.key, true) {
            match self.state.try_clone() {
                Some(state) => scripted::insert(entity, state),
                None => error!("{}: a state value cannot be cloned", decl.key),
            }
        }
    }

    fn detach(&self, entity: &mut EntityWorldMut) {
        scripted::remove(entity, &self.decl().key, false);
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }

    fn has_state(&self) -> bool {
        self.decl().has_state()
    }

    fn requires(&self) -> &[Arc<str>] {
        &self.decl().requires
    }

    fn required_decls(&self) -> &[Arc<AspectDecl>] {
        self.module.as_ref().map_or(&[], |module| module.aspects())
    }

    /// Its fields by name, in declaration order, the state ones at their authored values.
    fn to_json(&self, registry: &TypeRegistry) -> Result<Value, BevyError> {
        let mut object = Map::new();
        for (field, value) in self.fields() {
            object.insert(field.name.to_string(), field_json(value, registry)?);
        }
        Ok(object.into())
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::entity::EntityCloner;
    use futures_lite::future::block_on;
    use panoply_saga::{compile_module, HostState, NoImports};
    use serde_json::json;

    use super::*;
    use crate::{
        exemplar::ExemplarData, interaction::tests::load_formula,
        script_components::missing_aspects, Exemplar, ScriptComponents,
    };

    const KEY: &str = "scripts::toggle_switch::ToggleSwitch";

    /// The aspects `src` declares, as the module `scripts/toggle_switch.saga`, with the editor's
    /// attributes.
    fn declare(src: &str) -> Vec<Arc<AspectDecl>> {
        let mut host = HostState::new();
        crate::annotations::add_editor_attributes(&mut host);
        block_on(compile_module(
            "scripts/toggle_switch.saga",
            src,
            &host,
            &NoImports,
        ))
        .unwrap()
        .aspects()
        .to_vec()
    }

    fn switch_decls() -> Vec<Arc<AspectDecl>> {
        declare(
            r#"#[accessor(toggle_switch)]
            aspect ToggleSwitch for Wall, Fixture {
                caption: String = "Use",
                state on: bool = false,
            }"#,
        )
    }

    fn on(world: &World, entity: Entity) -> Option<bool> {
        let (_, state) = scripted::get(world, entity, KEY, true);
        state?.get("on")?.downcast_ref::<bool>().copied()
    }

    #[test]
    fn values_come_from_the_file_or_the_declaration() {
        let decls = switch_decls();
        let registry = TypeRegistry::new();
        let aspect =
            ScriptedAspect::from_json(&decls[0], &json!({ "caption": "Pull" }), &registry, "t")
                .unwrap();
        assert_eq!(aspect.key(), KEY);
        let caption = aspect.config().get("caption").unwrap();
        assert_eq!(caption.downcast_ref::<String>().unwrap(), "Pull");
        let on = aspect.initial_state().get("on").unwrap();
        assert_eq!(on.downcast_ref::<bool>(), Some(&false));
        assert_eq!(
            aspect.to_json(&registry).unwrap(),
            json!({ "caption": "Pull", "on": false })
        );
        assert_eq!(
            format!("{aspect:?}"),
            r#"ToggleSwitch { caption: "Pull", on: false }"#
        );
        assert!(aspect.can_attach(InstanceKind::Fixture));
        assert!(!aspect.can_attach(InstanceKind::Actor));
    }

    #[test]
    fn a_field_changed_in_a_copy_leaves_the_original() {
        let decls = switch_decls();
        let registry = TypeRegistry::new();
        let aspect = ScriptedAspect::new(&decls[0], &registry).unwrap();
        let mut copy = aspect.clone();
        copy.field_mut("caption")
            .unwrap()
            .apply(&"Pull".to_string());
        copy.field_mut("on").unwrap().apply(&true);
        assert!(copy.field_mut("missing").is_none());
        assert_eq!(
            aspect.to_json(&registry).unwrap(),
            json!({ "caption": "Use", "on": false })
        );
        assert_eq!(
            copy.to_json(&registry).unwrap(),
            json!({ "caption": "Pull", "on": true })
        );
    }

    #[test]
    fn attributes_are_the_editors_annotations() {
        use bevy::annotations::{ReadOnly, Step, ValueRange};

        use crate::annotations::{AspectIcon, Precision};

        let decls = declare(
            r#"#[icon("editor/icons/lucide/lock.png")]
            aspect Lever for Wall {
                #[range(0, 2.5)] #[step(0.5)] #[precision(1)]
                reach: f32 = 1,
                #[read_only]
                state pulls: i32,
            }"#,
        );
        let icon = decls[0].attributes.get::<AspectIcon>().unwrap();
        assert_eq!(icon.0, "editor/icons/lucide/lock.png");
        let reach = &decls[0].field("reach").unwrap().attributes;
        assert_eq!(reach.get::<ValueRange>(), Some(&ValueRange(0.0..=2.5)));
        assert_eq!(reach.get::<Step>(), Some(&Step(0.5)));
        assert_eq!(reach.get::<Precision>().map(|places| places.0), Some(1));
        assert!(decls[0]
            .field("pulls")
            .unwrap()
            .attributes
            .contains::<ReadOnly>());
    }

    #[test]
    fn a_when_reads_the_state_an_aspect_attaches() {
        let decls = switch_decls();
        let registry = TypeRegistry::new();
        let aspect = ScriptedAspect::from_json(&decls[0], &json!({}), &registry, "t").unwrap();
        let host = HostState::new();
        let (when, _) = load_formula("!self.toggle_switch.on", &host, &decls);
        let enabled = |world: &World, entity| when.evaluate(world, &host, entity) == Some(true);
        let mut world = World::new();
        let mut entity = world.spawn_empty();
        aspect.attach(&mut entity);
        let entity = entity.id();
        assert!(enabled(&world, entity));

        scripted::get_mut(&mut world, entity, KEY, true)
            .unwrap()
            .get_mut("on")
            .unwrap()
            .apply(&true);
        assert!(!enabled(&world, entity));

        // Reapplying keeps the state; detaching removes the config and leaves the state.
        aspect.attach(&mut world.entity_mut(entity));
        assert_eq!(on(&world, entity), Some(true));
        aspect.detach(&mut world.entity_mut(entity));
        let entity_mut = world.entity_mut(entity);
        assert!(!scripted::contains(&entity_mut, KEY, false));
        assert!(scripted::contains(&entity_mut, KEY, true));

        // A clone of the entity has both parts.
        aspect.attach(&mut world.entity_mut(entity));
        let copy = world.spawn_empty().id();
        EntityCloner::build_opt_out(&mut world).clone_entity(entity, copy);
        assert_eq!(on(&world, copy), Some(true));

        // An entity without the aspect offers nothing.
        let bare = world.spawn_empty().id();
        assert!(!enabled(&world, bare));
    }

    #[test]
    fn saved_state_loads_after_the_module_changes() {
        let registry = TypeRegistry::new();
        let old = switch_decls();
        let state = ScriptedFields::new(old[0].clone(), true, vec![Box::new(true)]).unwrap();
        let saved = part_to_json(&state, &registry).unwrap();
        assert_eq!(saved, json!({ "on": true }));

        // A field added takes its default; one removed is dropped.
        let new = declare(
            r#"aspect ToggleSwitch for Wall {
                state on: bool = false,
                state presses: i32 = 7,
            }"#,
        );
        let loaded = part_from_json(&new[0], true, &saved, &registry, "save").unwrap();
        assert_eq!(
            loaded.get("on").unwrap().downcast_ref::<bool>(),
            Some(&true)
        );
        assert_eq!(
            loaded.get("presses").unwrap().downcast_ref::<i32>(),
            Some(&7)
        );

        let renamed = declare("aspect ToggleSwitch for Wall { state lit: bool }");
        let loaded = part_from_json(&renamed[0], true, &saved, &registry, "save").unwrap();
        assert_eq!(
            loaded.get("lit").unwrap().downcast_ref::<bool>(),
            Some(&false)
        );

        // A value of the wrong type keeps the field's default.
        let wrong = json!({ "on": "yes" });
        let loaded = part_from_json(&old[0], true, &wrong, &registry, "save").unwrap();
        assert_eq!(
            loaded.get("on").unwrap().downcast_ref::<bool>(),
            Some(&false)
        );
    }

    #[test]
    fn a_formula_needs_the_scripted_aspect_it_reads() {
        let decls = switch_decls();
        let registry = TypeRegistry::new();
        let (_, formulas) = load_formula("self.toggle_switch.on", &HostState::new(), &decls);
        let exemplar = |aspects: crate::AspectList| ExemplarData {
            kind: Some(InstanceKind::Wall),
            display_name: None,
            alias: Vec::new(),
            aspects,
            extends: None,
            is_abstract: false,
            formulas: formulas.clone(),
        };
        let exemplars = Assets::<Exemplar>::default();
        let components = ScriptComponents::default();
        let aspect = ScriptedAspect::from_json(&decls[0], &json!({}), &registry, "t").unwrap();
        let with = exemplar(vec![Arc::new(aspect)]);
        assert!(missing_aspects(&exemplars, &with, &components).is_empty());
        let without = exemplar(Vec::new());
        let missing = missing_aspects(&exemplars, &without, &components);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].accessor, "toggle_switch");
        assert_eq!(missing[0].aspect, KEY);
    }
}
