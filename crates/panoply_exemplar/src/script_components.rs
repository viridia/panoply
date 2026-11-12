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
    aspect: TypeId,
    aspect_name: &'static str,
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
                    aspect: TypeId::of::<A>(),
                    aspect_name: A::short_type_path(),
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
    pub(crate) accessor: &'static str,
    pub(crate) aspect: &'static str,
}

/// What `exemplar`'s formulas read from `self` that no aspect along its chain gives. A formula
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
                    aspect: component.aspect_name,
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
        interaction::tests::{lid_host, load_interactions, Lid},
        DetachAspect, InstanceType, RemoveComponent,
    };

    /// The aspect that gives an instance a [`Lid`].
    #[derive(Reflect, Clone, Default)]
    struct Lidded;

    impl Aspect for Lidded {
        fn name(&self) -> &str {
            "Lidded"
        }

        fn can_attach(&self, _meta_type: InstanceType) -> bool {
            true
        }

        fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
            static DETACH: RemoveComponent<Lid> = RemoveComponent::<Lid>::new();
            entity.insert_if_new(Lid::default());
            &DETACH
        }

        fn clone_boxed(&self) -> Box<dyn Aspect> {
            Box::new(self.clone())
        }
    }

    fn exemplar(
        aspects: Vec<Box<dyn Aspect>>,
        extends: Option<Handle<Exemplar>>,
        formulas: Vec<panoply_saga::FormulaRef>,
    ) -> ExemplarData {
        ExemplarData {
            meta_type: InstanceType::NONE,
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
        let (_, formulas) = load_interactions(
            r#"[{"id": "open", "caption": "", "when": "!self.lid.open", "action": "a"}]"#,
            &host,
        );
        let mut components = ScriptComponents::default();
        components.0.insert(
            TypeId::of::<Lid>(),
            ScriptComponent {
                aspect: TypeId::of::<Lidded>(),
                aspect_name: "Lidded",
                accessor: "lid",
            },
        );
        let mut exemplars = Assets::<Exemplar>::default();

        let lidded = exemplar(vec![Box::new(Lidded)], None, formulas.clone());
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
            vec![Box::new(Lidded)],
            None,
            Vec::new(),
        ))));
        let derived = exemplar(Vec::new(), Some(base), formulas);
        assert!(missing_aspects(&exemplars, &derived, &components).is_empty());
    }
}
