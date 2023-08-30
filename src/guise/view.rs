use bevy::{
    asset::{AssetPath, LoadState},
    ecs::system::Command,
    prelude::*,
    ui::FocusPolicy,
};
use bevy_trait_query::One;
use std::sync::Arc;

use crate::guise::style::ComputedStyle;

use super::{
    controller::Controller,
    controllers::DefaultController,
    path::relative_asset_path,
    template::{Template, TemplateNode, TemplateNodeList},
    StyleAsset,
};

/// Component that defines the root of a view hierarchy and a template invocation.
#[derive(Component, Default)]
pub struct ViewRoot {
    pub template: Handle<Template>,
}

/// Component that defines a ui element, and which can differentially update when the
/// template asset changes.
#[derive(Component, Default)]
pub struct ViewElement {
    /// Element id
    pub id: Option<String>,

    /// Reference to style element by name.
    pub styleset: Vec<String>,

    /// Cached handles for stylesets.
    pub styleset_handles: Vec<Handle<StyleAsset>>,

    /// Inline styles defined on this element.
    pub inline_style: Option<Arc<StyleAsset>>,

    /// ID of controller component associated with this element.
    pub controller: Option<String>,

    // pub controller_instance: Option<Arc<dyn Controller>>,
    // Class names used for style selectors.
    pub classes: Vec<String>,
}

impl ViewElement {
    pub fn element_id<'a>(&'a self) -> &'a str {
        match self.id {
            Some(ref id) => &id,
            None => "#unnamed",
        }
    }

    /// Calculate the "computed" style struct for this `ViewElement`.
    pub fn apply_base_styles(&self, computed: &mut ComputedStyle, assets: &Assets<StyleAsset>) {
        // if let Some(ref style_handle) = self.style {
        // style_handle.apply_to(computed);
        // }
    }

    pub fn apply_selected_styles(&self, computed: &mut ComputedStyle, class_names: &[&str]) {
        // if let Some(ref style_handle) = self.style {
        // style_handle.apply_selected_to(computed, class_names);
        // }
    }

    pub fn apply_inline_styles(&self, computed: &mut ComputedStyle) {
        if let Some(ref inline) = self.inline_style {
            inline.apply_to(computed);
        }
    }
}

pub struct InsertController {
    entity: Entity,
    controller: String,
}

/// Custom command to insert a Component by its type name. This is used for Controllers.
impl Command for InsertController {
    fn apply(self, world: &mut World) {
        let rcmp = {
            let types = world.resource::<AppTypeRegistry>().read();
            // TODO: Also lookup "full" name
            match types.get_with_short_name(&self.controller) {
                Some(controller_type) => {
                    // TODO: Not sure cloning the ReflectComponent is a good idea here,
                    // but needed to avoid borrowing World.
                    Some(controller_type.data::<ReflectComponent>().unwrap().clone())
                }
                None => None,
            }
        };

        if let Some(rcmp) = rcmp {
            let controller = rcmp.from_world(world);
            rcmp.insert(&mut world.entity_mut(self.entity), controller.as_ref());
        } else {
            println!("Controller type not found: [{}]", self.controller);
        }
    }
}

pub fn create_views(
    mut commands: Commands,
    mut root_query: Query<(Entity, Ref<ViewRoot>, Option<&Children>)>,
    mut view_query: Query<&mut ViewElement>,
    view_children_query: Query<&Children, With<ViewElement>>,
    // mut text_query: Query<&Text>,
    server: Res<AssetServer>,
    assets: Res<Assets<Template>>,
    mut ev_template: EventReader<AssetEvent<Template>>,
) {
    for ev in ev_template.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    match assets.get(handle) {
                        Some(template) => {
                            for (entity, view_root, children) in root_query.iter_mut() {
                                if view_root.template.eq(handle) {
                                    if let Some(ref element) = template.content {
                                        let root = reconcile_element(
                                            &mut commands,
                                            // &server,
                                            children.map(|list| list[0]),
                                            &element,
                                            &mut view_query,
                                            &view_children_query,
                                            &server,
                                            &asset_path,
                                        );
                                        commands.entity(entity).replace_children(&[root]);
                                    }
                                }
                            }
                        }

                        None => {
                            let status = server.get_load_state(handle);
                            warn!(
                                "Failure to load template: {:?}, status [{:?}]",
                                asset_path, status
                            );
                        }
                    }
                }
            }

            AssetEvent::Removed { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    warn!("Asset Removed: Template {:?}", asset_path);
                }
            }
        }
    }
}

/// Function to update the view hierarchy in response to changes to the templates and params.
/// This tries to preserve the existing view hierarchy (a bit like React's VDOM), but will destroy
/// and re-create entire sub-trees of entities if it feels that differential updates are too
/// complicated.
fn reconcile_element(
    commands: &mut Commands,
    view_entity: Option<Entity>,
    template_node: &Box<TemplateNode>,
    view_query: &mut Query<&mut ViewElement>,
    view_children_query: &Query<&Children, With<ViewElement>>,
    server: &AssetServer,
    asset_path: &AssetPath,
) -> Entity {
    match template_node.as_ref() {
        TemplateNode::Element(template) => {
            if let Some(view_entity) = view_entity {
                if let Ok(mut old_view) = view_query.get_mut(view_entity) {
                    if old_view.controller == template.controller {
                        // Update view element node with changed properties.
                        if old_view.id != template.id {
                            old_view.id = template.id.clone();
                        }

                        if !old_view.styleset.eq(&template.styleset) {
                            old_view.styleset = template.styleset.clone();
                            let mut handles: Vec<Handle<StyleAsset>> =
                                Vec::with_capacity(template.styleset.len());
                            for style_path in template.styleset.iter() {
                                handles
                                    .push(server.load(relative_asset_path(asset_path, &style_path)))
                            }
                            old_view.styleset_handles = handles;
                        }

                        if old_view.inline_style != template.inline_style {
                            old_view.inline_style = template.inline_style.clone();
                        }

                        // Visit and reconcile children
                        let old_children: &[Entity] = match view_children_query.get(view_entity) {
                            Ok(children) => children,
                            _ => &[],
                        };
                        let old_count = old_children.len();
                        let new_count = template.children.len();
                        let max_index = old_count.max(new_count);
                        let mut new_children: Vec<Entity> = Vec::with_capacity(new_count);
                        let mut children_changed = false;

                        for i in 0..max_index {
                            if i >= new_count {
                                // New list is smaller than the old list, so delete excess entities.
                                commands.entity(old_children[i]).despawn_recursive();
                                children_changed = true;
                            } else {
                                let old_child = if i < old_count {
                                    Some(old_children[i])
                                } else {
                                    None
                                };
                                let new_child = reconcile_element(
                                    commands,
                                    old_child,
                                    &template.children[i],
                                    view_query,
                                    view_children_query,
                                    &server,
                                    &asset_path,
                                );
                                if old_child != Some(new_child) {
                                    children_changed = true;
                                }
                                new_children.push(new_child)
                            }
                        }

                        if children_changed {
                            commands.entity(view_entity).replace_children(&new_children);
                        }

                        return view_entity;
                    }
                    commands.entity(view_entity).despawn_recursive();
                }
            }

            // Replace entire entity
            let mut handles: Vec<Handle<StyleAsset>> = Vec::with_capacity(template.styleset.len());
            for style_path in template.styleset.iter() {
                handles.push(server.load(relative_asset_path(asset_path, &style_path)))
            }

            let new_entity = commands
                .spawn((
                    ViewElement {
                        id: template.id.clone(),
                        styleset: template.styleset.clone(),
                        styleset_handles: handles,
                        inline_style: template.inline_style.clone(),
                        ..default()
                    },
                    NodeBundle {
                        background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                        border_color: Color::BLUE.into(),
                        focus_policy: FocusPolicy::Pass,
                        ..default()
                    },
                ))
                .id();

            // See if there's a controller for this ui node.
            if let Some(ref controller_id) = template.controller {
                // println!("Controller {}", controller_id);
                commands.add(InsertController {
                    entity: new_entity,
                    controller: controller_id.clone(),
                });
            } else {
                commands.entity(new_entity).insert(DefaultController);
            }

            // Visit and create children
            if !template.children.is_empty() {
                let child_entities = template
                    .children
                    .iter()
                    .map(|child| {
                        reconcile_element(
                            commands,
                            None,
                            child,
                            view_query,
                            view_children_query,
                            &server,
                            &asset_path,
                        )
                    })
                    .collect::<Vec<Entity>>();
                commands
                    .entity(new_entity)
                    .replace_children(&child_entities);
            }

            return new_entity;
        }

        TemplateNode::Text(text) => {
            todo!("Render Text")
        }

        TemplateNode::Fragment(text) => {
            todo!("Render Fragment")
        }
    }
}

/// Function to update the view hierarchy in response to changes to the templates and params.
/// This tries to preserve the existing view hierarchy (a bit like React's VDOM), but will destroy
/// and re-create entire sub-trees of entities if it feels that differential updates are too
/// complicated.
fn reconcile_template(
    commands: &mut Commands,
    // server: &AssetServer,
    // asset_path: &AssetPath,
    root: Entity,
    root_children: Option<&Children>,
    root_template_nodes: &TemplateNodeList,
    view_query: &mut Query<(&mut ViewElement, Option<&Children>)>,
) {
    // Use a queue to visit the tree; easier than trying to pass a borrowed query into a recursive
    // function.
    let mut to_visit = Vec::<(Entity, &TemplateNodeList)>::with_capacity(64);
    to_visit.push((root, &root_template_nodes));

    // Loop which compares the list of child template nodes with the existing child entities.
    while let Some((parent, parent_template_nodes)) = to_visit.pop() {
        // Logic is a bit complex here because the root has a different Component type than the
        // rest of the tree. Turn it into a list.
        let children: &[Entity] = if parent == root {
            match root_children {
                Some(ch) => ch,
                None => &[],
            }
        } else {
            match view_query.get(parent) {
                Ok((_, Some(ch))) => ch,
                _ => &[],
            }
        };

        let old_count = children.len();
        let new_count = parent_template_nodes.len();
        let max_index = old_count.max(new_count);
        let mut new_children: Vec<Entity> = Vec::with_capacity(new_count);
        let mut children_changed = false;

        for i in 0..max_index {
            if i >= new_count {
                // New list is smaller than the old list, so delete excess entities.
                commands.entity(children[i]).despawn_recursive();
                children_changed = true;
            } else {
                let template_node = &parent_template_nodes[i];
                match template_node.as_ref() {
                    TemplateNode::Element(elt) => {
                        // let style = elt.style;
                        if i < old_count {
                            let old_child = children[i];
                            match view_query.get(old_child) {
                                Ok((view, grand_children)) => {
                                    // Patch the existing node instead of replacing it, but only if the
                                    // controller hasn't changed. Otherwise, fall through and
                                    // destroy / re-create.
                                    if view.controller.eq(&elt.controller) {
                                        // println!("Patching VE {}", i);
                                        let mut changed = false;
                                        if !view.styleset.eq(&elt.styleset)
                                            || view.inline_style != elt.inline_style
                                        {
                                            changed = true;
                                        }

                                        // Replace view element node.
                                        // TODO: Mutate the view element in place rather than replacing
                                        // it. This will require splitting the query.
                                        if changed {
                                            commands.entity(old_child).insert((ViewElement {
                                                id: elt.id.clone(),
                                                styleset: elt.styleset.clone(),
                                                inline_style: elt.inline_style.clone(),
                                                ..default()
                                            },));
                                        }

                                        new_children.push(old_child);
                                        if !elt.children.is_empty() || grand_children.is_some() {
                                            to_visit.push((old_child, &elt.children));
                                        }

                                        continue;
                                    }
                                }

                                // Fall through and replace the entity.
                                Err(_) => {}
                            }

                            // println!("Replacing VE {}", i);
                            commands.entity(old_child).despawn_recursive();
                        }

                        // Create the new entity
                        let new_entity = commands
                            .spawn((
                                ViewElement {
                                    id: elt.id.clone(),
                                    styleset: elt.styleset.clone(),
                                    inline_style: elt.inline_style.clone(),
                                    ..default()
                                },
                                NodeBundle {
                                    background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                                    border_color: Color::BLUE.into(),
                                    focus_policy: FocusPolicy::Pass,
                                    ..default()
                                },
                            ))
                            .id();

                        // See if there's a controller for this ui node.
                        if let Some(ref controller_id) = elt.controller {
                            // println!("Controller {}", controller_id);
                            commands.add(InsertController {
                                entity: new_entity,
                                controller: controller_id.clone(),
                            });
                        } else {
                            commands.entity(new_entity).insert(DefaultController);
                        }

                        children_changed = true;
                        new_children.push(new_entity);
                        if elt.children.len() > 0 {
                            to_visit.push((new_entity, &elt.children));
                        }
                    }

                    TemplateNode::Text(text) => {
                        if i < old_count {
                            let old_child = children[i];
                            // println!("Replacing VE {}", i);
                            commands.entity(old_child).despawn_recursive();
                        }

                        // Create the new entity
                        let new_entity = commands
                            .spawn((TextBundle {
                                text: Text::from_section(
                                    text.content.clone(),
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

                        children_changed = true;
                        new_children.push(new_entity);
                    }

                    TemplateNode::Fragment(_frag) => {
                        panic!("Implement fragment")
                    }
                }
            }
        }

        if children_changed {
            commands.entity(parent).replace_children(&new_children);
        }
    }
}

// fn get_named_styles(
//     name: Option<&String>,
//     base_path: &AssetPath,
//     server: &AssetServer,
// ) -> Option<Handle<StyleAsset>> {
//     // Check if template has a 'style' attribute
//     name.map(|str| {
//         let style_path = relative_asset_path(&base_path, str);
//         // println!("Relative asset: {:?}", style_path);
//         server.load(style_path)
//     })
// }

pub fn attach_view_controllers(
    mut commands: Commands,
    query: Query<(Entity, &ViewElement, One<&dyn Controller>), Added<ViewElement>>,
) {
    for (entity, view, controller) in query.iter() {
        controller.attach(&mut commands, entity, view);
    }
}

/// One of two updaters for computing the ui node styles, this one uses asset events to detect
/// when a stylesheet is loaded or changed.
// pub fn update_view_styles(
//     mut commands: Commands,
//     query: Query<(Entity, &ViewElement, One<&dyn Controller>)>,
//     server: Res<AssetServer>,
//     assets: Res<Assets<PartialStyle>>,
//     mut ev_style: EventReader<AssetEvent<PartialStyle>>,
// ) {
//     for ev in ev_style.iter() {
//         match ev {
//             AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
//                 if let Some(asset_path) = server.get_handle_path(handle) {
//                     debug!("Asset Created/Modified: Style {:?}", asset_path);
//                 }

//                 for (entity, view, controller) in query.iter() {
//                     if let Some(ref style_handle) = view.style {
//                         if style_handle.eq(handle) {
//                             // println!("Updating styles for node: [{}]", view.element_id());
//                             controller.update_styles(&mut commands, entity, &view, &assets);
//                             commands.entity(entity).remove::<StyleHandlesChanged>();
//                             // view.set_changed();
//                         }
//                     }
//                 }
//             }

//             AssetEvent::Removed { handle } => {
//                 if let Some(asset_path) = server.get_handle_path(handle) {
//                     warn!("Asset Removed: Style {:?}", asset_path);
//                 }
//             }
//         }
//     }
// }

/// One of two updaters for computing the ui node styles, this one looks for a marker component
/// on the entity.
pub fn update_view_styles(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut ViewElement,
            One<&dyn Controller>,
            Option<&Parent>,
        ),
        Changed<ViewElement>,
    >,
    assets: Res<Assets<StyleAsset>>,
    server: Res<AssetServer>,
) {
    for (entity, view, controller, _parent) in query.iter_mut() {
        let ready =
            server.get_group_load_state(view.styleset_handles.iter().map(|handle| handle.id()));
        if ready == LoadState::Loaded {
            info!("{} styles ready", view.styleset_handles.len());
            controller.update_styles(&mut commands, entity, &view, &assets);
        } else {
            warn!("Styles not ready!");
        }
    }
}
