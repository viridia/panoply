use std::{path::PathBuf, sync::Arc};

use bevy::{asset::AssetPath, prelude::*};

use crate::guise::style::ComputedStyle;

use super::{
    partial_style::PartialStyle,
    template::{Template, TemplateNode},
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
    pub controller_key: Option<String>,
}

// impl Drop for ViewElement {
//     fn drop(&mut self) {
//         println!("View element dropped");
//     }
// }

pub fn create_views(
    mut commands: Commands,
    mut root_query: Query<(Entity, Ref<ViewRoot>, Option<&Children>)>,
    mut _elem_query: Query<(Entity, &ViewElement, Option<&Children>)>,
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
                            println!("Template Root Entity found: {:?}", asset_path);
                            reconcile_template(
                                &mut commands,
                                &server,
                                &asset_path,
                                template,
                                entity,
                                children,
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
    template: &Template,
    root_entity: Entity,
    children: Option<&Children>,
) {
    let path = asset_path.path();
    let tm_children: &[Entity] = match children {
        Some(ch) => ch,
        None => &[],
    };
    let mut new_children: Vec<Entity> = Vec::with_capacity(template.children.len());
    let max_index = tm_children.len().max(template.children.len());
    for i in 0..max_index {
        if i < tm_children.len() {
            if i < template.children.len() {
                // Compare and Patch
                println!("Patching: {:?}", path.display());
            } else {
                // Remove
                println!("Despawning: {:?}", path.display());
                commands.entity(tm_children[i]).despawn_recursive();
            }
        } else if i < template.children.len() {
            let ui_node = &template.children[i];
            println!("Spawning entity: {}", path.display());

            // Build the Ui bundle here - we need our own bundle type.
            // Recurse into template.
            // We need child entities
            // We need template params

            let style = get_named_styles(ui_node, asset_path, server);
            new_children.push(
                (*commands)
                    .spawn((
                        ViewElement {
                            style: style.clone(),
                            inline_styles: ui_node.inline_styles.clone(),
                            ..default()
                        },
                        NodeBundle {
                            background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                            border_color: Color::BLUE.into(),
                            ..default()
                        },
                    ))
                    .id(),
            )
        }
    }

    commands.entity(root_entity).replace_children(&new_children);
}

fn get_named_styles(
    ui_node: &TemplateNode,
    base_path: &AssetPath,
    server: &AssetServer,
) -> Option<Handle<PartialStyle>> {
    // Check if template has a 'style' attribute
    ui_node.attrs.get("style").map(|str| {
        let style_path = relative_asset_path(&base_path, str);
        println!("Relative asset: {:?}", style_path);
        server.load(style_path)
    })
}

pub fn update_view_styles(
    mut commands: Commands,
    query: Query<(Entity, Ref<ViewElement>)>,
    server: Res<AssetServer>,
    assets: Res<Assets<PartialStyle>>,
    mut ev_style: EventReader<AssetEvent<PartialStyle>>,
) {
    for ev in ev_style.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    println!("Style asset event {:?}", asset_path);
                }

                for (entity, view) in query.iter() {
                    if let Some(ref style_handle) = view.style {
                        if style_handle.eq(handle) {
                            if let Some(ps) = assets.get(handle) {
                                let mut computed = ComputedStyle::default();
                                ps.apply_to(&mut computed);
                                println!("Style updated 1");
                                commands.entity(entity).insert(computed.style);
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

    for (entity, view) in query.iter() {
        // if view.is_changed() {
        if let Some(ref style) = view.style {
            println!("Style load state: {:?}", server.get_load_state(style));
            if let Some(ps) = assets.get(&style) {
                let mut computed = ComputedStyle::default();
                ps.apply_to(&mut computed);
                println!("Style updated 2");
                commands.entity(entity).insert(computed.style);
            }
        }
        // }
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
