use std::{path::PathBuf, sync::Arc};

use bevy::{
    asset::{AssetPath, LoadState},
    prelude::*,
};

use crate::guise::style::ComputedStyle;

use super::{
    style::PartialStyle,
    template::{Template, TemplateNodeList},
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
    /// Reference to style element by name
    pub style: Option<Handle<PartialStyle>>,

    /// Inline styles for this view element
    pub inline_styles: Option<Arc<PartialStyle>>,

    /// ID of controller component associated with this element.
    /// Note: when this changes, we will need to remove the old controller components (somehow).
    pub controller: Option<String>,
}

/// Marker that signals when a component's stylesheet handles have changed.
#[derive(Component, Default)]
pub struct StyleHandlesChanged;

pub fn create_views(
    mut commands: Commands,
    mut root_query: Query<(Entity, Ref<ViewRoot>, Option<&Children>)>,
    mut view_query: Query<(&ViewElement, Option<&Children>)>,
    server: Res<AssetServer>,
    assets: Res<Assets<Template>>,
    mut ev_template: EventReader<AssetEvent<Template>>,
) {
    for ev in ev_template.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    let template = assets.get(handle).unwrap();
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
            }

            AssetEvent::Removed { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    println!("Template asset removed {:?}", asset_path);
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
    view_query: &mut Query<(&ViewElement, Option<&Children>)>,
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

        for i in 0..max_index {
            if i >= new_count {
                // New list is smaller than the old list, so delete excess entities.
                commands.entity(children[i]).despawn_recursive();
            } else {
                let template_node = &parent_template_nodes[i];
                let style = get_named_styles(template_node.attrs.get("style"), asset_path, server);
                if i < old_count {
                    let old_child = children[i];
                    match view_query.get(old_child) {
                        Ok((view, grand_children)) => {
                            // Patch the existing node instead of replacing it, but only if the
                            // controller hasn't changed. Otherwise, fall through and
                            // destroy / re-create.
                            if view.controller.eq(&template_node.controller) {
                                let mut changed = false;
                                if !view.style.eq(&style)
                                    || view.inline_styles != template_node.inline_styles
                                {
                                    changed = true;
                                }

                                // Replace view element node.
                                if changed {
                                    commands.entity(old_child).insert((
                                        ViewElement {
                                            style: style.clone(),
                                            inline_styles: template_node.inline_styles.clone(),
                                            ..default()
                                        },
                                        StyleHandlesChanged,
                                    ));
                                }

                                new_children.push(old_child);
                                if !template_node.children.is_empty() || grand_children.is_some() {
                                    to_visit.push((old_child, &template_node.children));
                                }

                                continue;
                            }
                        }
                        Err(_) => {}
                    }

                    commands.entity(old_child).despawn_recursive();
                }

                // let style = get_named_styles(ui_node, asset_path, server);
                let new_entity = (*commands)
                    .spawn((
                        ViewElement {
                            style: style.clone(),
                            inline_styles: template_node.inline_styles.clone(),
                            ..default()
                        },
                        StyleHandlesChanged,
                        NodeBundle {
                            background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                            border_color: Color::BLUE.into(),
                            ..default()
                        },
                    ))
                    .id();
                new_children.push(new_entity);
                if template_node.children.len() > 0 {
                    to_visit.push((new_entity, &template_node.children));
                }
            }
        }

        commands.entity(parent).replace_children(&new_children);
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
    query: Query<(Entity, &ViewElement)>,
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

                for (entity, view) in query.iter() {
                    update_computed(&mut commands, entity, view, &assets);
                    // if let Some(ref style_handle) = view.style {
                    //     if style_handle.eq(handle) {
                    //         if let Some(ps) = assets.get(handle) {
                    //             let mut computed = ComputedStyle::default();
                    //             ps.apply_to(&mut computed);
                    //             if let Some(ref inline) = view.inline_styles {
                    //                 inline.apply_to(&mut computed);
                    //             }
                    //             println!("Style updated 1");
                    //             commands
                    //                 .entity(entity)
                    //                 .insert(computed.style)
                    //                 .remove::<StyleHandlesChanged>();
                    //         }
                    //     }
                    // }
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
    query: Query<(Entity, &ViewElement), With<StyleHandlesChanged>>,
    server: Res<AssetServer>,
    assets: Res<Assets<PartialStyle>>,
) {
    for (entity, view) in query.iter() {
        if let Some(ref style_handle) = view.style {
            if server.get_load_state(style_handle) != LoadState::Loaded {
                continue;
            }
        }
        update_computed(&mut commands, entity, view, &assets);
    }
}

fn update_computed(
    commands: &mut Commands,
    entity: Entity,
    view: &ViewElement,
    assets: &Assets<PartialStyle>,
) {
    let mut computed = ComputedStyle::default();
    if let Some(ref style_handle) = view.style {
        if let Some(ps) = assets.get(&style_handle) {
            ps.apply_to(&mut computed);
        }
    }
    // TODO: Controllers
    if let Some(ref inline) = view.inline_styles {
        inline.apply_to(&mut computed);
    }

    let mut e = commands.entity(entity);
    match computed.background_color {
        Some(color) => e.insert(BackgroundColor(color)),
        None => e.remove::<BackgroundColor>(),
    };
    match computed.border_color {
        Some(color) => e.insert(BorderColor(color)),
        None => e.remove::<BorderColor>(),
    };

    e.insert((computed.style,)).remove::<StyleHandlesChanged>();
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
