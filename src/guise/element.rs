use anyhow::anyhow;

use bevy::{asset::LoadContext, reflect::Reflect};

use super::{
    expr::Expr,
    from_ast::{FromAst, ReflectFromAst},
    RenderContext, RenderOutput, Renderable,
};

#[derive(Debug, Default, Clone, Reflect)]
#[type_path = "panoply::guise::Element"]
#[reflect(FromAst)]
pub struct Element {
    /// List of expressions for Style
    #[reflect(ignore)]
    pub style: Vec<Expr>,
    // / Styles loaded and dereferenced
    // #[reflect(ignore)]
    // pub style_objects: Vec<Arc<ElementStyle>>,
    // pub rebuild_style: bool,
    // // ID of this node
    // pub id: Option<String>,

    // // Attached controller
    // pub controller: Option<String>,

    // // List of child nodes
    // #[serde(default)]
    // pub children: TemplateNodeList,
    // // special attrs
    // // each / if / match
}

impl FromAst for Element {
    fn from_ast<'a>(
        members: bevy::utils::HashMap<String, super::expr::Expr>,
        _load_context: &'a mut LoadContext,
    ) -> Result<Self, anyhow::Error> {
        let mut style = Vec::<Expr>::new();
        for (key, value) in members.iter() {
            match key.as_str() {
                "style" => match value {
                    Expr::List(items) => {
                        style.reserve(items.len());
                        for item in items.iter() {
                            match item {
                                Expr::Null => todo!(),
                                Expr::Ident(_) => todo!(),
                                Expr::List(_) => todo!(),
                                Expr::Object(_) => {
                                    style.push(item.clone());
                                }
                                Expr::Asset(_) => todo!(),
                                Expr::Var(_) => todo!(),
                                _ => {
                                    return Err(anyhow!("Invalid style object."));
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(anyhow!("Expected list for style: '{}'", key));
                    }
                },
                _ => return Err(anyhow!("Invalid property: '{}'", key)),
            }
        }
        Ok(Self { style })
    }
}

impl Renderable for Element {
    fn render(&self, output: &RenderOutput, context: &mut RenderContext) -> RenderOutput {
        if let RenderOutput::Node(elt_entity) = *output {
            if let Some(mut element) = context.get_mut_view_element(elt_entity) {
                todo!("Element exists");
            }
        }
        todo!()
    }
}
