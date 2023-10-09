use std::sync::Arc;

use bevy::{asset::LoadContext, prelude::*, reflect::FromType, utils::HashMap};

use super::expr::Expr;

pub trait FromAst: std::fmt::Debug + Sized {
    // Construct a new instance of the object given a map of properties.
    fn from_ast<'a, 'b>(
        props: HashMap<String, Expr>,
        load_context: &'a mut LoadContext,
    ) -> Result<Self, anyhow::Error>;
}

#[derive(Clone)]
pub struct ReflectFromAst {
    pub from_ast: fn(
        props: HashMap<String, Expr>,
        load_context: &mut LoadContext,
    ) -> Result<Arc<dyn Reflect>, anyhow::Error>,
}

impl ReflectFromAst {
    pub fn from_ast<'a>(
        &self,
        props: HashMap<String, Expr>,
        load_context: &'a mut LoadContext,
    ) -> Result<Arc<dyn Reflect>, anyhow::Error> {
        (self.from_ast)(props, load_context)
    }
}

impl<T: FromAst + Reflect> FromType<T> for ReflectFromAst {
    fn from_type() -> Self {
        Self {
            from_ast: |props, lc| Ok(Arc::new(T::from_ast(props, lc)?)),
        }
    }
}
