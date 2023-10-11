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
            Expr::Bool(true) => self.render_text(prev, "true"),
            Expr::Ident(_) => todo!(),
            Expr::Text(text) => self.render_text(prev, text),
            Expr::Number(num) => self.render_text(prev, &num.to_string()),
            Expr::Length(_) => todo!(),
            Expr::List(_) => todo!(),
            Expr::Color(_) => todo!(),
            Expr::Asset(_) => todo!(),
            Expr::Var(_) => todo!(),
            Expr::Style(_) => {
                panic!("Object is not renderable");
            }
            Expr::Template(template) => self.render(prev, &template.expr, props),
            Expr::Renderable(renderable) => renderable.render(renderable, prev, self, props),
        }
    }

    // Render text
    pub fn render_text(&mut self, prev: &RenderOutput, str: &str) -> RenderOutput {
        if let RenderOutput::Node(text_entity) = prev {
            if let Ok(mut old_text) = self.query_text.get_mut(*text_entity) {
                old_text.sections.clear();
                old_text.sections.push(TextSection {
                    value: str.to_string(),
                    style: TextStyle { ..default() },
                });
                return RenderOutput::Node(*text_entity);
            }
        }

        let new_entity = self
            .commands
            .spawn((TextBundle {
                text: Text::from_section(str.clone(), TextStyle { ..default() }),
                // TextStyle {
                //     font_size: 40.0,
                //     color: Color::rgb(0.9, 0.9, 0.9),
                //     ..Default::default()
                // },
                // background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                // border_color: Color::BLUE.into(),
                // focus_policy: FocusPolicy::Pass,
                ..default()
            },))
            .id();

        return RenderOutput::Node(new_entity);
    }
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
