use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::HashMap;

use super::computed::ComputedStyle;
use super::view_element::ViewElement;
use super::{Expr, GuiseAsset, RenderOutput};
use std::fmt::Debug;
use std::sync::Arc;

pub type RenderProps = Arc<HashMap<String, Expr>>;

#[derive(SystemParam)]
pub struct RenderContext<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub query_elements: Query<'w, 's, &'static mut ViewElement>,
    pub query_text: Query<'w, 's, &'static mut Text>,
    pub server: Res<'w, AssetServer>,
    pub assets: Res<'w, Assets<GuiseAsset>>,
}

impl<'w, 's> RenderContext<'w, 's> {
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
            Expr::AssetPath(path) => panic!("Unloaded asset path: {}", path),
            Expr::Asset(asset) => match self.assets.get(asset) {
                Some(GuiseAsset(expr)) => self.render(prev, &expr.clone(), props),
                None => RenderOutput::Empty,
            },
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
    // TODO: State
    fn render<'a>(
        &self,
        template: &Arc<dyn Renderable>,
        output: &RenderOutput,
        context: &'a mut RenderContext,
        props: &RenderProps,
    ) -> RenderOutput;

    fn adjust_styles(
        &self,
        _output: &RenderOutput,
        _props: &RenderProps,
        _style: &mut ComputedStyle,
    ) {
    }
}
