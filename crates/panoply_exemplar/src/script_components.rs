//! Components that formulas read through accessors (`self.lock`), and the check that an
//! exemplar's formulas read only what its aspects give an instance.

use std::any::TypeId;

use bevy::{platform::collections::HashMap, prelude::*, reflect::Typed};
use panoply_saga::SagaHostBuilder;

use crate::{exemplar::ExemplarData, Aspect, Exemplar};

/// Each component formulas can read, by type, with the aspect that gives an instance it.
#[derive(Resource, Default)]
pub struct ScriptComponents(HashMap<TypeId, ScriptComponent>);

struct ScriptComponent {
    /// The key of the aspect that gives an instance the component.
    aspect: &'static str,
    accessor: &'static str,
}

/// Registers components for formulas to read.
pub trait ScriptComponentAppExt {
    /// Lets formulas read component `C` as `entity.accessor`, as `self.lock` reads `LockState`.
    /// Aspect `A` gives an instance `C`, so a formula that reads `C` from `self` is an error in
    /// an exemplar without `A`. Call it while plugins build, after `SagaPlugin`.
    fn register_script_component<A: Aspect + TypePath, C: Component + Reflect + Typed>(
        &mut self,
        accessor: &'static str,
    ) -> &mut Self;
}

impl ScriptComponentAppExt for App {
    fn register_script_component<A: Aspect + TypePath, C: Component + Reflect + Typed>(
        &mut self,
        accessor: &'static str,
    ) -> &mut Self {
        self.world_mut()
            .get_resource_mut::<SagaHostBuilder>()
            .expect("register_script_component runs while plugins build, after SagaPlugin")
            .0
            .add_component::<C>(accessor);
        self.world_mut()
            .get_resource_or_init::<ScriptComponents>()
            .0
            .insert(
                TypeId::of::<C>(),
                ScriptComponent {
                    aspect: A::short_type_path(),
                    accessor,
                },
            );
        self
    }
}

/// A formula reading a component from `self` that its exemplar gives no instance.
#[derive(Debug, PartialEq)]
pub(crate) struct MissingAspect<'a> {
    pub(crate) formula: &'a str,
    pub(crate) accessor: &'a str,
    /// The key of the aspect that gives an instance the component.
    pub(crate) aspect: &'a str,
}

/// What `exemplar`'s formulas read from `self` that no aspect along its chain gives, native or
/// scripted. A formula
/// is checked only in the exemplar that writes it, since one that extends it never loses an
/// aspect.
pub(crate) fn missing_aspects<'a>(
    exemplars: &Assets<Exemplar>,
    exemplar: &'a ExemplarData,
    components: &ScriptComponents,
) -> Vec<MissingAspect<'a>> {
    let mut missing = Vec::new();
    for formula in exemplar.formulas.iter() {
        let Some(module) = formula.module() else {
            continue;
        };
        for id in module.self_components() {
            let Some(component) = components.0.get(id) else {
                continue;
            };
            if !Exemplar::has_aspect(exemplars, exemplar, component.aspect) {
                missing.push(MissingAspect {
                    formula: formula.source(),
                    accessor: component.accessor,
                    aspect: component.aspect,
                });
            }
        }
        for aspect in module.self_aspects() {
            if !Exemplar::has_aspect(exemplars, exemplar, &aspect.key) {
                missing.push(MissingAspect {
                    formula: formula.source(),
                    accessor: aspect.accessor.as_deref().unwrap_or_default(),
                    aspect: &aspect.key,
                });
            }
        }
    }
    missing
}

/// Checks each exemplar's formulas once it and its chain have loaded.
pub(crate) fn check_script_components(
    mut events: MessageReader<AssetEvent<Exemplar>>,
    exemplars: Res<Assets<Exemplar>>,
    components: Option<Res<ScriptComponents>>,
    server: Res<AssetServer>,
) {
    let Some(components) = components else {
        events.clear();
        return;
    };
    for event in events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        let Some(exemplar) = exemplars.get(*id) else {
            continue;
        };
        for missing in missing_aspects(&exemplars, &exemplar.0, &components) {
            let path = server
                .get_path(*id)
                .map_or_else(|| format!("{id}"), |path| path.to_string());
            error!(
                "{path}: `{}` reads self.{}, which needs a {} aspect",
                missing.formula, missing.accessor, missing.aspect
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        interaction::tests::{lid_host, load_formula, Lid},
        AspectList, InstanceKind, NativeAspect,
    };

    /// The aspect that gives an instance a [`Lid`], its state, and has no config.
    #[derive(Reflect, Clone, Default)]
    struct Lidded;

    impl NativeAspect for Lidded {
        type Config = ();

        fn config(&self) {}

        fn attach_state(&self, entity: &mut EntityWorldMut) {
            entity.insert_if_new(Lid::default());
        }

        fn can_attach(&self, _kind: InstanceKind) -> bool {
            true
        }
    }

    fn exemplar(
        aspects: AspectList,
        extends: Option<Handle<Exemplar>>,
        formulas: Vec<panoply_saga::FormulaRef>,
    ) -> ExemplarData {
        ExemplarData {
            kind: None,
            display_name: None,
            alias: Vec::new(),
            aspects,
            extends,
            is_abstract: false,
            formulas,
        }
    }

    #[test]
    fn formulas_need_the_aspects_of_what_they_read_from_self() {
        let host = lid_host();
        let (_, formulas) = load_formula("!self.lid.open", &host, &[]);
        let mut components = ScriptComponents::default();
        components.0.insert(
            TypeId::of::<Lid>(),
            ScriptComponent {
                aspect: "Lidded",
                accessor: "lid",
            },
        );
        let mut exemplars = Assets::<Exemplar>::default();

        let lidded = exemplar(vec![Arc::new(Lidded)], None, formulas.clone());
        assert!(missing_aspects(&exemplars, &lidded, &components).is_empty());

        let lidless = exemplar(Vec::new(), None, formulas.clone());
        assert_eq!(
            missing_aspects(&exemplars, &lidless, &components),
            [MissingAspect {
                formula: "!self.lid.open",
                accessor: "lid",
                aspect: "Lidded",
            }]
        );

        // An aspect inherited along the chain counts.
        let base = exemplars.add(Exemplar(Arc::new(exemplar(
            vec![Arc::new(Lidded)],
            None,
            Vec::new(),
        ))));
        let derived = exemplar(Vec::new(), Some(base), formulas);
        assert!(missing_aspects(&exemplars, &derived, &components).is_empty());
    }
}
