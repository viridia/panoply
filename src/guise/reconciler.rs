use std::sync::Arc;

use bevy::{asset::AssetPath, prelude::*, ui::FocusPolicy, utils::HashMap};

use super::{
    path::relative_asset_path, template::*, template_output::TemplateOutput,
    view_element::ViewElement, ElementStyle, Expr,
};

pub struct Reconciler<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    query_elements: &'a mut Query<'w, 's, &'a mut ViewElement>,
    query_text: &'a mut Query<'w, 's, &'a mut Text>,
    server: &'a AssetServer,
    assets: &'a Assets<TemplateAsset>,
}

impl<'a, 'w, 's> Reconciler<'a, 'w, 's> {
    pub fn visit_expr(
        &mut self,
        expr: &Expr,
        output: &TemplateOutput,
        base_path: &AssetPath,
        props: &Arc<HashMap<String, TemplateExpr>>,
    ) -> TemplateOutput {
        match expr {
            Expr::Null => TemplateOutput::Empty,
            Expr::Bool(false) => TemplateOutput::Empty,
            Expr::Bool(true) => {
                todo!()
            }
            Expr::Ident(_) => todo!(),
            Expr::Number(_) => todo!(),
            Expr::Length(_) => todo!(),
            Expr::List(_) => todo!(),
            Expr::Color(_) => todo!(),
            Expr::Object(obj) => {
                todo!()
            }
            Expr::Asset(_) => todo!(),
            Expr::Var(_) => todo!(),
        }
    }

    pub fn visit_node(
        &mut self,
        node: &Expr,
        output: &TemplateOutput,
        base_path: &AssetPath,
        props: &Arc<HashMap<String, TemplateExpr>>,
    ) -> TemplateOutput {
        match node.as_ref() {
            TemplateNode::Element(template) => {
                if let TemplateOutput::Node(elt_entity) = *output {
                    if let Ok(mut element) = self.query_elements.get_mut(elt_entity) {
                        if element.controller == template.controller {
                            // Update view element node with changed properties.
                            if element.id != template.id {
                                element.id = template.id.clone();
                            }

                            // if !element.styleset_handles.eq(&template.styleset_handles) {
                            //     element.styleset_handles = template.styleset_handles.clone();
                            // }

                            // if element.inline_style != template.inline_style {
                            //     element.inline_style = template.inline_style.clone();
                            // }

                            // Visit and reconcile children
                            let new_count = template.children.len();
                            let mut children: Vec<TemplateOutput> =
                                vec![TemplateOutput::Empty; template.children.len()];
                            let mut children_changed = false;

                            for i in 0..element.children.len() {
                                if i < new_count {
                                    children[i] = element.children[i].clone()
                                } else {
                                    element.children[i].despawn_recursive(self.commands);
                                    children_changed = true;
                                }
                            }

                            for i in 0..new_count {
                                let new_child = self.visit_node(
                                    &template.children[i],
                                    &children[i],
                                    &base_path,
                                    props,
                                );
                                if children[i] != new_child {
                                    children[i] = new_child;
                                    children_changed = true;
                                }
                            }

                            if children_changed {
                                let count = children.iter().map(|child| child.count()).sum();
                                let mut flat = Vec::<Entity>::with_capacity(count);
                                children.iter().for_each(|child| child.flatten(&mut flat));
                                self.commands.entity(elt_entity).replace_children(&flat);
                                if let Ok(mut element) = self.query_elements.get_mut(elt_entity) {
                                    element.children = children;
                                }
                            }

                            // We patched the old entity, so just return the same entity id.
                            return TemplateOutput::Node(elt_entity);
                        }
                    }

                    // Delete the old entity as we are going to re-create it.
                    self.commands.entity(elt_entity).despawn_recursive();
                }

                let mut handles: Vec<Handle<ElementStyle>> =
                    Vec::with_capacity(template.styleset.len());
                template.styleset.iter().for_each(|ss| {
                    handles.push(self.server.load(relative_asset_path(base_path, ss)));
                });

                // Visit and create children
                let mut children = vec![TemplateOutput::Empty; template.children.len()];
                let mut flat = Vec::<Entity>::new();
                if !template.children.is_empty() {
                    for i in 0..template.children.len() {
                        children[i] =
                            self.visit_node(&template.children[i], &children[i], &base_path, props)
                    }
                    let count = children.iter().map(|child| child.count()).sum();
                    flat.reserve(count);
                    children.iter().for_each(|child| child.flatten(&mut flat));
                }

                let new_entity = self
                    .commands
                    .spawn((
                        ViewElement {
                            template: (*node).clone(),
                            id: template.id.clone(),
                            style: template.styleset_handles.clone(),
                            controller: template.controller.clone(),
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

                // See if there's a controller for this ui node.
                // if let Some(ref controller_id) = template.controller {
                //     info!("Creating controller {}", controller_id);
                //     self.commands.add(InsertController {
                //         entity: new_entity,
                //         controller: controller_id.clone(),
                //     });
                // } else {
                //     self.commands.entity(new_entity).insert(DefaultController);
                // }

                return TemplateOutput::Node(new_entity);
            }

            TemplateNode::Text(template) => {
                if let TemplateOutput::Node(text_entity) = *output {
                    if let Ok(mut old_text) = self.query_text.get_mut(text_entity) {
                        old_text.sections.clear();
                        old_text.sections.push(TextSection {
                            value: template.content.clone(),
                            style: TextStyle { ..default() },
                        });
                        return TemplateOutput::Node(text_entity);
                    }
                }

                let new_entity = self
                    .commands
                    .spawn((TextBundle {
                        text: Text::from_section(
                            template.content.clone(),
                            TextStyle { ..default() },
                        ),
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

                return TemplateOutput::Node(new_entity);
            }

            TemplateNode::Invoke(invoke) => {
                let template = self.assets.get(&invoke.template_handle).unwrap();
                info!(
                    "Invoking template: {} with {} params",
                    invoke.template,
                    invoke.params.len()
                );
                self.visit_node(
                    template.content.as_ref().unwrap(),
                    output,
                    &base_path, // Should be path to invoked template
                    &invoke.params,
                )
            }

            TemplateNode::Fragment(_template) => {
                todo!("Render Fragment")
            }

            // TemplateNode::ParamRef(name) => {
            //     if let Some(val) = props.as_ref().get(&name.param) {
            //         todo!("Render ParamVal");
            //     } else {
            //         error!("Unknown param [{}]", name.param);
            //         info!("Num params {}", props.as_ref().len());
            //         return TemplateOutput::Empty;
            //     }
            // }
            TemplateNode::Expression(expr) => {
                let ctx = EvalContext {};
                expr.render(&ctx)
            }
        }
    }

    /// Render a text string node.
    pub fn render_text(&mut self, text: &str, output: &TemplateOutput) -> TemplateOutput {
        todo!();
    }
}
