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
    // // ID of this node
    // pub id: Option<String>,

    // // Attached controller
    // pub controller: Option<String>,
    /// List of child nodes
    #[reflect(ignore)]
    pub children: Vec<Expr>,
    // // special attrs
    // // each / if / match
}

impl FromAst for Element {
    fn from_ast<'a>(
        members: bevy::utils::HashMap<String, super::expr::Expr>,
        _load_context: &'a mut LoadContext,
    ) -> Result<Expr, anyhow::Error> {
        let mut style = Vec::<Expr>::new();
        let mut children = Vec::<Expr>::new();
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
                "children" => match value {
                    Expr::List(items) => {
                        children.reserve(items.len());
                        for item in items.iter() {
                            match item {
                                Expr::Style(_) => {
                                    return Err(anyhow!("Invalid child object."));
                                }
                                _ => {
                                    children.push(item.clone());
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(anyhow!("Expected list for children: '{}'", key));
                    }
                },
                _ => return Err(anyhow!("Invalid property: '{}'", key)),
            }
        }
        Ok(Expr::Renderable(Arc::new(Self { style, children })))
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
            if let Ok(mut view_element) = context.query_elements.get_mut(elt_entity) {
                // Check template pointer equality
                if std::ptr::eq(view_element.template.as_ref(), template.as_ref()) {
                    // Update styles
                    if view_element.style != self.style {
                        view_element.style = self.style.clone();
                    }

                    // Visit and render children
                    let new_count = self.children.len();
                    let mut children: Vec<RenderOutput> =
                        vec![RenderOutput::Empty; self.children.len()];
                    let mut children_changed = false;

                    for i in 0..self.children.len() {
                        if i < new_count {
                            children[i] = view_element.children[i].clone()
                        } else {
                            view_element.children[i].despawn_recursive(context.commands);
                            children_changed = true;
                        }
                    }

                    for i in 0..new_count {
                        let new_child = context.render(&children[i], &self.children[i], props);
                        if children[i] != new_child {
                            children[i] = new_child;
                            children_changed = true;
                        }
                    }

                    if children_changed {
                        let count = children.iter().map(|child| child.count()).sum();
                        let mut flat = Vec::<Entity>::with_capacity(count);
                        children.iter().for_each(|child| child.flatten(&mut flat));
                        context.commands.entity(elt_entity).replace_children(&flat);
                        if let Ok(mut element) = context.query_elements.get_mut(elt_entity) {
                            element.children = children;
                        }
                    }

                    // We patched the old entity, so just return the same entity id.
                    return RenderOutput::Node(elt_entity);
                }
            }
        }

        // Visit and create children
        let mut children = vec![RenderOutput::Empty; self.children.len()];
        let mut flat = Vec::<Entity>::new();
        if !self.children.is_empty() {
            for i in 0..self.children.len() {
                children[i] = context.render(&children[i], &self.children[i], props);
            }
            let count = children.iter().map(|child| child.count()).sum();
            flat.reserve(count);
            children.iter().for_each(|child| child.flatten(&mut flat));
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
                    children,
                    classes: Vec::new(),
                    props: props.clone(),
                },
                NodeBundle {
                    focus_policy: FocusPolicy::Pass,
                    visibility: Visibility::Visible,
                    ..default()
                },
            ))
            .replace_children(&flat)
            .id();

        // if let Some(mut element) = context.get_mut_view_element(elt_entity) {
        //     todo!("Element exists");
        // }
        return RenderOutput::Node(new_entity);
    }
}
