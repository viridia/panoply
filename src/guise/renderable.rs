use bevy::prelude::*;
use bevy::utils::HashMap;

use super::view_element::ViewElement;
use super::{Expr, RenderOutput};
use std::fmt::Debug;
use std::sync::Arc;

pub type RenderProps = Arc<HashMap<String, Expr>>;

pub struct RenderContext<'r0, 'w0, 's0, 'r1, 'w1, 's1, 'r2, 'w2, 's2> {
    pub(crate) commands: &'r0 mut Commands<'w0, 's0>,
    pub(crate) query_elements: &'r1 mut Query<'w1, 's1, &'static mut ViewElement>,
    pub(crate) query_text: &'r2 mut Query<'w2, 's2, &'static mut Text>,
}

impl<'r0, 'w0, 's0, 'r1, 'w1, 's1, 'r2, 'w2, 's2>
    RenderContext<'r0, 'w0, 's0, 'r1, 'w1, 's1, 'r2, 'w2, 's2>
{
    pub fn render(
        &mut self,
        prev: &RenderOutput,
        expr: &Expr,
        props: &RenderProps,
    ) -> RenderOutput {
        match expr {
            Expr::Null => RenderOutput::Empty,
            Expr::Bool(false) => RenderOutput::Empty,
            Expr::Bool(true) => todo!(),
            Expr::Ident(_) => todo!(),
            Expr::Number(_) => todo!(),
            Expr::Length(_) => todo!(),
            Expr::List(_) => todo!(),
            Expr::Color(_) => todo!(),
            Expr::Object(_) => todo!(),
            Expr::Asset(_) => todo!(),
            Expr::Var(_) => todo!(),
            Expr::Style(_) => {
                panic!("Object is not renderable");
            }
            Expr::Template(template) => self.render(prev, &template.expr, props),
            Expr::Renderable(renderable) => renderable.render(renderable, prev, self, props),
        }
    }

    // pub fn get_view_element(&self, entity: Entity) -> Option<&ViewElement> {
    //     self.query_elements.get(entity).ok()
    // }

    // pub fn get_mut_view_element(&mut self, entity: Entity) -> Option<Mut<ViewElement>> {
    //     self.query_elements.get_mut(entity).ok()
    // }
}

pub trait Renderable: Sync + Send + Debug {
    fn render<'a>(
        &self,
        template: &Arc<dyn Renderable>,
        output: &RenderOutput,
        context: &'a mut RenderContext,
        props: &RenderProps,
    ) -> RenderOutput;
}
