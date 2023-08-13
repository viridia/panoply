use std::path::PathBuf;

use bevy::{asset::AssetPath, prelude::*};

use super::{style::PartialStyle, template::Template};

#[derive(Component, Default)]
pub struct ViewRoot {
    pub template: Handle<Template>,
}

#[derive(Component, Default)]
pub struct ViewElement {
    // Need a way to reference the template node.
    // This needs to be stable across file changes.
    pub style: Option<Handle<PartialStyle>>,
}

pub struct VViewContext<'w, 's> {
    pub world: &'w mut World,
    pub commands: &'w mut Commands<'w, 's>,
    server: Res<'w, AssetServer>,
    templates: Res<'w, Assets<Template>>,
    styles: Res<'w, Assets<PartialStyle>>,
}

impl<'w, 's> VViewContext<'w, 's> {
    pub fn create() {}
}

#[derive(Default)]
pub struct LocalViewContext {
    handle: Handle<Template>,
}

impl LocalViewContext {
    pub fn create_root(&mut self, path: &str) {
        // self.handle = server.l
    }
}

// pub fn create_control_panel(
//     server: Res<AssetServer>,
//     templates: Res<Assets<Template>>,
//     styles: Res<Assets<PartialStyle>>,
//     mut ctx: Local<LocalViewContext>,
// ) {
//     let handle_cpanel: Handle<Template> = server.load("editor/ui/test.guise.xml#main");
//     let handle_panel: Handle<PartialStyle> = server.load("editor/ui/test.guise.xml#panel");
//     let ui_cpanel = templates.get(&handle_cpanel);

//     ctx.create_root("editor/ui/test.guise.xml#main");
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
                    let path = asset_path.path();
                    for (entity, root, children) in root_query.iter_mut() {
                        if root.template.eq(handle) {
                            println!("Root found: {}", path.display());
                            let tm_children: &[Entity] = match children {
                                Some(ch) => ch,
                                None => &[],
                            };
                            let mut new_children: Vec<Entity> =
                                Vec::with_capacity(template.children.len());
                            let max_index = tm_children.len().max(template.children.len());
                            for i in 0..max_index {
                                if i < tm_children.len() {
                                    if i < template.children.len() {
                                        // Compare Patch
                                    } else {
                                        // Remove
                                        commands.entity(tm_children[i]).despawn_recursive();
                                    }
                                } else if i < template.children.len() {
                                    let ui_node = &template.children[i];
                                    let style: Option<Handle<PartialStyle>> = match ui_node
                                        .attrs
                                        .get("style")
                                    {
                                        Some(str) => Some(server.load(if str.starts_with('#') {
                                            AssetPath::new_ref(asset_path.path(), Some(str))
                                        } else {
                                            AssetPath::from(str)
                                        })),

                                        None => None,
                                    };
                                    // Add new child
                                    new_children.push(
                                        commands.spawn(ViewElement { style, ..default() }).id(),
                                    )
                                }
                            }

                            commands.entity(entity).replace_children(&new_children);
                            // Build the Ui bundle here.
                            // Recurse into template.
                            // We need child entities
                            // We need params
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

pub fn update_view_styles(
    mut commands: Commands,
    mut query: Query<(&ViewElement)>,
    assets: Res<Assets<PartialStyle>>,
    server: Res<AssetServer>,
    mut ev_style: EventReader<AssetEvent<PartialStyle>>,
) {
    for ev in ev_style.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(asset_path) = server.get_handle_path(handle) {
                    println!("SS found: {}", asset_path.path().display());
                }
                for view in query.iter() {
                    if let Some(ref style_handle) = view.style {
                        if style_handle.eq(handle) {
                            println!("Style updated")
                        }
                    }
                }
            }
            _ => {}
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
        // assert_eq!(
        //     relative_asset_path(&base, "../martin#dave"),
        //     AssetPath::from("martin#dave")
        // );
        // assert_eq!(StyleAttr::parse_val("1").unwrap(), Val::Px(1.));
        // assert_eq!(StyleAttr::parse_val("1px").unwrap(), Val::Px(1.));
        // assert_eq!(StyleAttr::parse_val("1vw").unwrap(), Val::Vw(1.));
        // assert_eq!(StyleAttr::parse_val("1vh").unwrap(), Val::Vh(1.));
        // assert_eq!(StyleAttr::parse_val("1.1px").unwrap(), Val::Px(1.1));

        // assert!(StyleAttr::parse_val("1.1bad").is_err());
        // assert!(StyleAttr::parse_val("bad").is_err());
        // assert!(StyleAttr::parse_val("1.1.1bad").is_err());
    }
}
