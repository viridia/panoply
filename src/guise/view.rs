use bevy::{
    asset::{AssetPath, LoadState},
    ecs::system::Command,
    prelude::*,
    ui::FocusPolicy,
};
use bevy_trait_query::One;
use std::sync::Arc;

use super::{
    controller::Controller,
    controllers::DefaultController,
    path::relative_asset_path,
    template::{TemplateAsset, TemplateNode},
    StyleAsset,
};

/// Component that defines the root of a view hierarchy and a template invocation.
#[derive(Component, Default)]
pub struct ViewRoot {
    pub template: Handle<TemplateAsset>,
    pub entity: Option<Entity>,
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

    // pub fn apply_selected_styles(&self, computed: &mut ComputedStyle, class_names: &[&str]) {
    //     // if let Some(ref style_handle) = self.style {
    //     // style_handle.apply_selected_to(computed, class_names);
    //     // }
    // }
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
    mut root_query: Query<&mut ViewRoot>,
    mut view_query: Query<&mut ViewElement>,
    mut text_query: Query<&mut Text>,
    view_children_query: Query<&Children, With<ViewElement>>,
    server: Res<AssetServer>,
    assets: Res<Assets<TemplateAsset>>,
    mut ev_template: EventReader<AssetEvent<TemplateAsset>>,
) {
    for ev in ev_template.read() {
        match ev {
            AssetEvent::Added { id }
            | AssetEvent::LoadedWithDependencies { id }
            | AssetEvent::Modified { id } => {
                info!("Template event: {:?}", ev);
                if let Some(asset_path) = server.get_path(*id) {
                    match assets.get(*id) {
                        Some(template) => {
                            for mut view_root in root_query.iter_mut() {
                                if view_root.template.id().eq(id) {
                                    // println!("create_views: {} {:?}", asset_path, ev);
                                    if let Some(ref element) = template.content {
                                        let root = reconcile_element(
                                            &mut commands,
                                            view_root.entity,
                                            &element,
                                            &mut view_query,
                                            &view_children_query,
                                            &mut text_query,
                                            &server,
                                            &asset_path,
                                        );
                                        if view_root.entity != Some(root) {
                                            view_root.entity = Some(root);
                                        }
                                    }
                                }
                            }
                        }

                        None => {
                            let status = server.load_state(*id);
                            warn!(
                                "Failure to load template: {:?}, status [{:?}]",
                                asset_path, status
                            );
                        }
                    }
                }
            }

            AssetEvent::Removed { id } => {
                if let Some(asset_path) = server.get_path(*id) {
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
    text_query: &mut Query<&mut Text>,
    server: &AssetServer,
    asset_path: &AssetPath,
) -> Entity {
    match template_node.as_ref() {
        TemplateNode::Element(template) => {
            if let Some(elt_entity) = view_entity {
                if let Ok(mut old_view) = view_query.get_mut(elt_entity) {
                    if old_view.controller == template.controller {
                        // Update view element node with changed properties.
                        if old_view.id != template.id {
                            old_view.id = template.id.clone();
                        }

                        if !old_view.styleset.eq(&template.styleset) {
                            old_view.styleset = template.styleset.clone();
                            let mut handles: Vec<Handle<StyleAsset>> =
                                Vec::with_capacity(old_view.styleset.len());
                            old_view.styleset.iter().for_each(|ss| {
                                handles.push(server.load(relative_asset_path(asset_path, ss)));
                            });
                            old_view.styleset_handles = handles;
                        }

                        if old_view.inline_style != template.inline_style {
                            old_view.inline_style = template.inline_style.clone();
                        }

                        // Visit and reconcile children
                        let old_children: &[Entity] = match view_children_query.get(elt_entity) {
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
                                    text_query,
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
                            commands.entity(elt_entity).replace_children(&new_children);
                        }

                        // We patched the old entity, so just return the same entity id.
                        return elt_entity;
                    }
                }

                // Delete the old entity as we are going to re-create it.
                info!("Deleting old entity");
                commands.entity(elt_entity).despawn_recursive();
            }

            let mut handles: Vec<Handle<StyleAsset>> = Vec::with_capacity(template.styleset.len());
            template.styleset.iter().for_each(|ss| {
                handles.push(server.load(relative_asset_path(asset_path, ss)));
            });

            let new_entity = commands
                .spawn((
                    ViewElement {
                        id: template.id.clone(),
                        styleset: template.styleset.clone(),
                        styleset_handles: handles,
                        inline_style: template.inline_style.clone(),
                        controller: template.controller.clone(),
                        ..default()
                    },
                    NodeBundle {
                        focus_policy: FocusPolicy::Pass,
                        visibility: Visibility::Visible,
                        ..default()
                    },
                ))
                .id();

            // See if there's a controller for this ui node.
            if let Some(ref controller_id) = template.controller {
                info!("Creating controller {}", controller_id);
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
                            text_query,
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

        TemplateNode::Text(template) => {
            if let Some(text_entity) = view_entity {
                if let Ok(mut old_text) = text_query.get_mut(text_entity) {
                    old_text.sections.clear();
                    old_text.sections.push(TextSection {
                        value: template.content.clone(),
                        style: TextStyle { ..default() },
                    });
                    return text_entity;
                }
            }

            let new_entity = commands
                .spawn((TextBundle {
                    text: Text::from_section(template.content.clone(), TextStyle { ..default() }),
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

            return new_entity;
        }

        TemplateNode::Fragment(_template) => {
            todo!("Render Fragment")
        }
    }
}

pub fn attach_view_controllers(
    mut commands: Commands,
    query: Query<(Entity, &ViewElement, One<&dyn Controller>), Added<ViewElement>>,
) {
    for (entity, view, controller) in query.iter() {
        controller.attach(&mut commands, entity, view);
    }
}

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
    mut ev_style: EventReader<AssetEvent<StyleAsset>>,
) {
    for (entity, view, controller, _parent) in query.iter_mut() {
        let ready = view
            .styleset_handles
            .iter()
            .all(|handle| server.load_state(handle) == LoadState::Loaded);
        if ready {
            // info!("{} styles ready", view.styleset_handles.len());
            for handle in view.styleset_handles.iter() {
                let st = server.load_state(handle);
                if st != LoadState::Loaded {
                    error!("You lied: load state is {:?}", st);
                } else {
                    let st = assets.get(handle);
                    if st.is_none() {
                        error!(
                            "Failed to load stylesheet: {:?}",
                            server.get_path(handle).unwrap()
                        )
                    }
                }
            }

            controller.update_styles(&mut commands, entity, &view, &assets);
        } else {
            warn!("Styles not ready!");
        }
    }

    for ev in ev_style.read() {
        match ev {
            AssetEvent::Added { id }
            | AssetEvent::LoadedWithDependencies { id }
            | AssetEvent::Modified { id } => {
                if let Some(asset_path) = server.get_path(*id) {
                    info!("Asset Created/Modified: Style {:?}", asset_path);
                }

                for (entity, view, controller, _parent) in query.iter() {
                    if view.styleset_handles.iter().any(|h| h.id() == *id) {
                        // println!("Found handle!");
                        controller.update_styles(&mut commands, entity, &view, &assets);
                        // commands.entity(entity).remove::<StyleHandlesChanged>();
                    }
                    // if let Some(ref style_handle) = view.style {
                    //     if style_handle.eq(handle) {
                    //         // println!("Updating styles for node: [{}]", view.element_id());
                    //         controller.update_styles(&mut commands, entity, &view, &assets);
                    //         commands.entity(entity).remove::<StyleHandlesChanged>();
                    //         // view.set_changed();
                    //     }
                    // }
                }
            }

            AssetEvent::Removed { id } => {
                if let Some(asset_path) = server.get_path(*id) {
                    warn!("Asset Removed: Style {:?}", asset_path);
                }

                for (_entity, view, _controller, _parent) in query.iter() {
                    for style_handle in view.styleset_handles.iter() {
                        if style_handle.id() == *id {
                            println!("That was still being used!");
                        }
                    }
                    // if let Some(ref style_handle) = view.style {
                    //     if style_handle.eq(handle) {
                    //         // println!("Updating styles for node: [{}]", view.element_id());
                    //         controller.update_styles(&mut commands, entity, &view, &assets);
                    //         commands.entity(entity).remove::<StyleHandlesChanged>();
                    //         // view.set_changed();
                    //     }
                    // }
                }
            }
        }
    }
}
