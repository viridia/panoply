use bevy::{
    asset::{AssetPath, LoadState},
    ecs::system::Command,
    prelude::*,
    ui::FocusPolicy,
};
use bevy_trait_query::One;
use std::{path::PathBuf, sync::Arc};

use crate::guise::style::ComputedStyle;

use super::{
    controller::Controller,
    controllers::DefaultController,
    style::PartialStyle,
    template::{Template, TemplateNode, TemplateNodeList},
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

    /// Reference to style element by name
    pub style: Option<Handle<PartialStyle>>,

    /// Inline styles for this view element
    pub inline_styles: Option<Arc<PartialStyle>>,

    /// ID of controller component associated with this element.
    pub controller: Option<String>,
    // pub controller_instance: Option<Arc<dyn Controller>>,
    pub classes: Vec<String>,
}

impl ViewElement {
    /// Calculate the "computed" style struct for this `ViewElement`.
    pub fn compute_style(&self, computed: &mut ComputedStyle, assets: &Assets<PartialStyle>) {
        // TODO: Style state selectors
        if let Some(ref style_handle) = self.style {
            if let Some(ps) = assets.get(&style_handle) {
                ps.apply_to(computed);
            }
        }

        if let Some(ref inline) = self.inline_styles {
            inline.apply_to(computed);
        }
    }

    /// Calculate the "computed" style struct for this `ViewElement`.
    pub fn apply_base_styles(&self, computed: &mut ComputedStyle, assets: &Assets<PartialStyle>) {
        if let Some(ref style_handle) = self.style {
            if let Some(ps) = assets.get(&style_handle) {
                ps.apply_to(computed);
            }
        }
    }

    pub fn apply_selected_styles(
        &self,
        computed: &mut ComputedStyle,
        assets: &Assets<PartialStyle>,
        class_names: &[&str],
    ) {
        if let Some(ref style_handle) = self.style {
            if let Some(ps) = assets.get(&style_handle) {
                ps.apply_selected_to(computed, class_names);
            }
        }
    }

    pub fn apply_inline_styles(&self, computed: &mut ComputedStyle) {
        if let Some(ref inline) = self.inline_styles {
            inline.apply_to(computed);
        }
    }
}

/// Marker that signals when a component's stylesheet handles have changed.
#[derive(Component, Default)]
pub struct StyleHandlesChanged;

pub struct InsertController {
    entity: Entity,
    controller: String,
}

/// Custom command to insert a Component by its type name.
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
    mut view_query: Query<(&mut ViewElement, Option<&Children>)>,
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
                                    reconcile_template(
                                        &mut commands,
                                        &server,
                                        &asset_path,
                                        entity,
                                        children,
                                        &template.children,
                                        &mut view_query,
                                    );
                                }
                            }
                        }

                        None => {
                            warn!("Failure to load asset: {:?}", asset_path);
                        }
                    }
                }
            }

            AssetEvent::Removed { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    info!("Template asset removed {:?}", asset_path);
                }
            }
        }
    }
}

/// Function to update the view hierarchy in response to changes to the templates and params.
/// This tries to preserve the existing view hierarchy (a bit like React's VDOM), but will destroy
/// and re-create entire sub-trees of entities if it feels that differential updates are too
/// complicated.
fn reconcile_template(
    commands: &mut Commands,
    server: &AssetServer,
    asset_path: &AssetPath,
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
                        let style = get_named_styles(elt.attrs.get("style"), asset_path, server);
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
                                        if !view.style.eq(&style)
                                            || view.inline_styles != elt.inline_styles
                                        {
                                            changed = true;
                                        }

                                        // Replace view element node.
                                        // TODO: Mutate the view element in place rather than replacing
                                        // it. This will require splitting the query.
                                        if changed {
                                            commands.entity(old_child).insert((
                                                ViewElement {
                                                    id: elt.id.clone(),
                                                    style: style.clone(),
                                                    inline_styles: elt.inline_styles.clone(),
                                                    ..default()
                                                },
                                                StyleHandlesChanged,
                                            ));
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
                                    style: style.clone(),
                                    inline_styles: elt.inline_styles.clone(),
                                    ..default()
                                },
                                StyleHandlesChanged,
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
                            println!("Controller {}", controller_id);
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
                            // match view_query.get(old_child) {
                            //     Ok((view, grand_children)) => {
                            //         // Patch the existing node instead of replacing it, but only if the
                            //         // controller hasn't changed. Otherwise, fall through and
                            //         // destroy / re-create.
                            //         if view.controller.eq(&elt.controller) {
                            //             // println!("Patching VE {}", i);
                            //             let mut changed = false;
                            //             if !view.style.eq(&style)
                            //                 || view.inline_styles != elt.inline_styles
                            //             {
                            //                 changed = true;
                            //             }

                            //             // Replace view element node.
                            //             // TODO: Mutate the view element in place rather than replacing
                            //             // it. This will require splitting the query.
                            //             if changed {
                            //                 commands.entity(old_child).insert((
                            //                     ViewElement {
                            //                         style: style.clone(),
                            //                         inline_styles: elt.inline_styles.clone(),
                            //                         ..default()
                            //                     },
                            //                     StyleHandlesChanged,
                            //                 ));
                            //             }

                            //             new_children.push(old_child);
                            //             if !elt.children.is_empty() || grand_children.is_some() {
                            //                 to_visit.push((old_child, &elt.children));
                            //             }

                            //             continue;
                            //         }
                            //     }

                            //     // Fall through and replace the entity.
                            //     Err(_) => {}
                            // }

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

fn get_named_styles(
    name: Option<&String>,
    base_path: &AssetPath,
    server: &AssetServer,
) -> Option<Handle<PartialStyle>> {
    // Check if template has a 'style' attribute
    name.map(|str| {
        let style_path = relative_asset_path(&base_path, str);
        // println!("Relative asset: {:?}", style_path);
        server.load(style_path)
    })
}

pub fn update_view_styles(
    mut commands: Commands,
    query: Query<(Entity, &ViewElement, One<&dyn Controller>)>,
    server: Res<AssetServer>,
    assets: Res<Assets<PartialStyle>>,
    mut ev_style: EventReader<AssetEvent<PartialStyle>>,
) {
    for ev in ev_style.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    println!("Style asset created/modified {:?}", asset_path);
                }

                for (entity, view, controller) in query.iter() {
                    if let Some(ref style_handle) = view.style {
                        if style_handle.eq(handle) {
                            println!(
                                "Updating styles from handle: {}",
                                match view.id {
                                    Some(ref id) => id.clone(),
                                    None => "#unnamed".to_string(),
                                }
                            );
                            controller.attach(&commands, entity, view);
                            controller.update_styles(&mut commands, entity, &view, &assets);
                            // view.set_changed();
                        }
                    }
                }
            }

            AssetEvent::Removed { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    println!("Style asset removed {:?}", asset_path);
                }
            }
        }
    }
}

pub fn update_view_style_handles(
    mut commands: Commands,
    query: Query<(Entity, &ViewElement, One<&dyn Controller>), With<StyleHandlesChanged>>,
    server: Res<AssetServer>,
    assets: Res<Assets<PartialStyle>>,
) {
    for (entity, view, controller) in query.iter() {
        // Don't update style if stylesheet isn't loaded.
        if let Some(ref style_handle) = view.style {
            if server.get_load_state(style_handle) != LoadState::Loaded {
                continue;
            }
        }
        controller.update_styles(&mut commands, entity, &view, &assets);
    }
}

/// Resolves a relative asset path. The relative path can be one of:
/// * An absolute path e.g. `foo/bar#fragment`
/// * A path starting with './' or '../', e.g. `./bar#fragment`, in which case it is resolved
///   relative to the current directory.
/// * Just a label, `#fragment`.
fn relative_asset_path<'a>(base: &'a AssetPath<'a>, relative_path: &'a str) -> AssetPath<'a> {
    if relative_path.starts_with('#') {
        AssetPath::new_ref(base.path(), Some(&relative_path[1..]))
    } else if relative_path.starts_with("./") || relative_path.starts_with("../") {
        let mut rpath = relative_path;
        let mut fpath = PathBuf::from(base.path());
        if !fpath.pop() {
            panic!("Can't compute relative path");
        }
        loop {
            if rpath.starts_with("./") {
                rpath = &rpath[2..];
            } else if rpath.starts_with("../") {
                rpath = &rpath[3..];
                if !fpath.pop() {
                    panic!("Can't compute relative path");
                }
            } else {
                break;
            }
        }
        fpath.push(rpath);
        // Note: converting from a string causes AssetPath to look for the separator, while
        // passing fpath directly does not.
        AssetPath::from(String::from(fpath.to_str().unwrap()))
    } else {
        AssetPath::from(relative_path)
    }
}

// pub fn display_asset_path(path: AssetPath) -> String {
//     // path.path().
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path() {
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            relative_asset_path(&base, "joe/next"),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            relative_asset_path(&base, "#dave"),
            AssetPath::from("alice/bob#dave")
        );
        assert_eq!(
            relative_asset_path(&base, "./martin#dave"),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            relative_asset_path(&base, "../martin#dave"),
            AssetPath::from("martin#dave")
        );
    }
}
