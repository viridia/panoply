use std::sync::Arc;

use anyhow::anyhow;

use bevy::{asset::LoadContext, prelude::*, reflect::Reflect, ui::FocusPolicy};

use crate::guise::view_element::ViewElement;

use super::{
    expr::Expr,
    from_ast::{FromAst, ReflectFromAst},
    RenderContext, RenderOutput, RenderProps, Renderable,
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
    ) -> Result<Expr, anyhow::Error> {
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
                                Expr::Style(_) => {
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
        Ok(Expr::Renderable(Arc::new(Self { style })))
    }
}

impl Renderable for Element {
    fn render(
        &self,
        template: &Arc<dyn Renderable>,
        output: &RenderOutput,
        context: &mut RenderContext,
        props: &RenderProps,
    ) -> RenderOutput {
        // Get previous output
        if let RenderOutput::Node(elt_entity) = *output {
            // Node exists already, check for diff...
            if let Ok(view_element) = context.query_elements.get(elt_entity) {
                // Check template pointer equality
                if std::ptr::eq(view_element.template.as_ref(), template.as_ref()) {
                    println!("Same template as before...")
                }
            }
        }

        let new_entity = context
            .commands
            .spawn((
                ViewElement {
                    template: template.clone(),
                    // id: template.id.clone(),
                    id: None,
                    style: self.style.clone(),
                    // styleset_handles: template.styleset_handles.clone(),
                    // controller: template.controller.clone(),
                    presenter: None,
                    children: Vec::new(),
                    classes: Vec::new(),
                    props: props.clone(),
                },
                NodeBundle {
                    focus_policy: FocusPolicy::Pass,
                    visibility: Visibility::Visible,
                    ..default()
                },
            ))
            // .replace_children(&flat)
            .id();

        // if let Some(mut element) = context.get_mut_view_element(elt_entity) {
        //     todo!("Element exists");
        // }
        return RenderOutput::Node(new_entity);
    }
}
