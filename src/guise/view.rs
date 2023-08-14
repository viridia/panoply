use std::{path::PathBuf, sync::Arc};

use bevy::{
    asset::{AssetPath, LoadState},
    prelude::*,
};

use crate::guise::style::ComputedStyle;

use super::{
    partial_style::PartialStyle,
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
    mut view_query: Query<&mut ViewElement, Option<&Children>>,
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
                            // println!("Template Root Entity found: {:?}", asset_path);
                            reconcile_template(
                                &mut commands,
                                &server,
                                &asset_path,
                                &template.children,
                                entity,
                                children,
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
    template_node_list: &TemplateNodeList,
    parent: Entity,
    children: Option<&Children>,
    view_query: &mut Query<&mut ViewElement, Option<&Children>>,
) {
    // let path = asset_path.path();
    let old_count = if children.is_some() {
        children.unwrap().len()
    } else {
        0
    };
    let new_count = template_node_list.len();
    let max_index = old_count.max(new_count);
    let mut new_children: Vec<Entity> = Vec::with_capacity(new_count);

    for i in 0..max_index {
        if i >= new_count {
            // New list is smaller than the old list, so delete some entities.
            println!("Despawning: #{}", i);
            commands.entity(children.unwrap()[i]).despawn_recursive();
        } else {
            let template_node = &template_node_list[i];
            let style = get_named_styles(template_node.attrs.get("style"), asset_path, server);
            if i < old_count {
                let old_child = children.unwrap()[i];
                match view_query.get_mut(old_child) {
                    Ok(mut view) => {
                        // Patch the existing node instead of replacing it, but only if the controller
                        // hasn't changed. Otherwise, fall through and destroy / re-create.
                        if view.controller.eq(&template_node.controller) {
                            let mut changed = false;
                            if !view.style.eq(&style) {
                                view.style = style.clone();
                                changed = true;
                            }
                            // TODO
                            // if !view.inline_styles.eq(template_node.inline_styles) {
                            //     view.inline_styles = template_node.inline_styles.clone();
                            //     changed = true;
                            // }
                            view.inline_styles = template_node.inline_styles.clone();

                            if changed {
                                commands.entity(old_child).insert(StyleHandlesChanged);
                            }

                            new_children.push(old_child);

                            println!("Patching: #{}", i);
                            // Recurse and reconcile children of this node.
                            // reconcile_template(
                            //     commands,
                            //     server,
                            //     asset_path,
                            //     &template_node.children,
                            //     child_entity,
                            //     &mut elem_query,
                            //     node_children,
                            // );
                            continue;
                        }
                    }
                    Err(_) => {}
                }

                println!("Despawning: #{}", i);
                commands.entity(old_child).despawn_recursive();
            }

            println!("Spawning entity: #{}", i);

            // Build the Ui bundle here - we need our own bundle type.
            // Recurse into template.
            // We need child entities
            // We need template params

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
            // if template_node.children.len() > 0 {
            //     to_visit.push(ToVisit {
            //         entity: new_entity,
            //         template: &template_node.children,
            //     })
            // }
        }
    }

    commands.entity(parent).replace_children(&new_children);
}

// /// Function to update the view hierarchy in response to changes to the templates and params.
// /// This tries to preserve the existing view hierarchy (a bit like React's VDOM), but will destroy
// /// and re-create entire sub-trees of entities if it feels that differential updates are too
// /// complicated.
// fn reconcile_template(
//     commands: &mut Commands,
//     server: &AssetServer,
//     asset_path: &AssetPath,
//     template_node_list: &TemplateNodeList,
//     parent: Entity,
//     elem_query: &mut Query<(&mut ViewElement, Option<&Children>)>,
//     children: Option<&Children>,
// ) {
//     let path = asset_path.path();
//     let mut new_children: Vec<Entity> = Vec::with_capacity(template_node_list.len());
//     let old_count = if children.is_some() {
//         children.unwrap().len()
//     } else {
//         0
//     };
//     let new_count = template_node_list.len();
//     let max_index = old_count.max(new_count);

//     for i in 0..max_index {
//         // New list is smaller than the old list, so delete some entities.
//         if i >= new_count {
//             println!("Despawning: {:?}", path.display());
//             commands.entity(children.unwrap()[i]).despawn_recursive();
//             continue;
//         }

//         let template_node = &template_node_list[i];
//         let style = get_named_styles(template_node, asset_path, server);
//         if i < old_count {
//             let child_entity = children.unwrap()[i];
//             match elem_query.get_mut(child_entity) {
//                 Ok((mut view, node_children)) => {
//                     // Patch the existing node instead of replacing it, but only if the controller
//                     // hasn't changed. Otherwise, fall through and destroy / re-create.
//                     if view.controller.eq(&template_node.controller) {
//                         view.style = style.clone();
//                         view.inline_styles = template_node.inline_styles.clone();
//                         new_children.push(child_entity);
//                         println!("Patching: {:?}/{}", path.display(), i);
//                         // Recurse and reconcile children of this node.
//                         // reconcile_template(
//                         //     commands,
//                         //     server,
//                         //     asset_path,
//                         //     &template_node.children,
//                         //     child_entity,
//                         //     &mut elem_query,
//                         //     node_children,
//                         // );
//                         continue;
//                     }
//                 }
//                 Err(_) => {}
//             }
//         }

//         // If we're not keeping the old entity, then despawn it.
//         if i < old_count {
//             println!("Despawning: {:?}/{}", path.display(), i);
//             commands.entity(children.unwrap()[i]).despawn_recursive();
//         }

//         if i < new_count {
//             let ui_node = &template_node_list[i];
//             println!("Spawning: {:?}/{}", path.display(), i);

//             // TODO: Build the Ui bundle here - we need our own bundle type.
//             // We need template params

//             let style = get_named_styles(ui_node, asset_path, server);
//             let new_entity = (*commands)
//                 .spawn((
//                     ViewElement {
//                         style: style.clone(),
//                         inline_styles: ui_node.inline_styles.clone(),
//                         ..default()
//                     },
//                     NodeBundle {
//                         background_color: Color::rgb(0.65, 0.75, 0.65).into(),
//                         border_color: Color::BLUE.into(),
//                         ..default()
//                     },
//                 ))
//                 .id();

//             new_children.push(new_entity);
//             // reconcile_template(
//             //     commands,
//             //     server,
//             //     asset_path,
//             //     &template_node.children,
//             //     new_entity,
//             //     &mut elem_query,
//             //     None,
//             // );
//         }
//     }

//     // Update the parent's list of child ids.
//     commands.entity(parent).replace_children(&new_children);

//     // for (i, child) in new_children.iter().enumerate() {
//     //     match elem_query.get(*child) {
//     //         Ok((_, node_children)) => {
//     //             let template_node = &template_node_list[i];
//     //             reconcile_template(
//     //                 commands,
//     //                 server,
//     //                 asset_path,
//     //                 &template_node.children,
//     //                 *child,
//     //                 &mut elem_query,
//     //                 node_children,
//     //             );
//     //         }
//     //         Err(_) => {}
//     //     }
//     // }
// }

fn get_named_styles(
    name: Option<&String>,
    base_path: &AssetPath,
    server: &AssetServer,
) -> Option<Handle<PartialStyle>> {
    // Check if template has a 'style' attribute
    name.map(|str| {
        let style_path = relative_asset_path(&base_path, str);
        println!("Relative asset: {:?}", style_path);
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
                    if let Some(ref style_handle) = view.style {
                        if style_handle.eq(handle) {
                            if let Some(ps) = assets.get(handle) {
                                let mut computed = ComputedStyle::default();
                                ps.apply_to(&mut computed);
                                println!("Style updated 1");
                                commands
                                    .entity(entity)
                                    .insert(computed.style)
                                    .remove::<StyleHandlesChanged>();
                            }
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
    query: Query<(Entity, &ViewElement), With<StyleHandlesChanged>>,
    server: Res<AssetServer>,
    assets: Res<Assets<PartialStyle>>,
) {
    for (entity, view) in query.iter() {
        if let Some(ref style_handle) = view.style {
            let loaded = server.get_load_state(style_handle);
            // println!("Style load state: {:?}", loaded);
            if matches!(loaded, LoadState::Loaded) {
                if let Some(ps) = assets.get(&style_handle) {
                    let mut computed = ComputedStyle::default();
                    ps.apply_to(&mut computed);
                    println!("Style updated 2");
                    commands
                        .entity(entity)
                        .insert(computed.style)
                        .remove::<StyleHandlesChanged>();
                }
            }
        } else {
            commands
                .entity(entity)
                .remove::<StyleHandlesChanged>()
                .remove::<Style>();
        }
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
