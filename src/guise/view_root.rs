use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use super::{view_element::ViewElement, Expr, GuiseAsset, RenderContext, RenderOutput};

/// Component that defines the root of a view hierarchy and a template invocation.
#[derive(Component, Default)]
pub struct ViewRoot {
    pub template: Handle<GuiseAsset>,

    /// Generated list of entities
    out: RenderOutput,

    /// Template properties
    props: Arc<HashMap<String, Expr>>,
}

impl ViewRoot {
    pub fn new(template: Handle<GuiseAsset>) -> Self {
        Self {
            template,
            ..default()
        }
    }
}

#[derive(Component, Default)]
pub struct RebuildView;

pub fn render_views(
    mut commands: Commands,
    mut root_query: Query<&mut ViewRoot>,
    mut element_query: Query<&'static mut ViewElement>,
    mut text_query: Query<&'static mut Text>,
    server: Res<AssetServer>,
    assets: Res<Assets<GuiseAsset>>,
    mut ev_template: EventReader<AssetEvent<GuiseAsset>>,
) {
    for ev in ev_template.read() {
        match ev {
            AssetEvent::Added { id }
            | AssetEvent::LoadedWithDependencies { id }
            | AssetEvent::Modified { id } => {
                // info!("Guise asset event: {:?}", ev);
                match assets.get(*id) {
                    Some(asset) => {
                        for mut view_root in root_query.iter_mut() {
                            if view_root.template.id().eq(id) {
                                // commands.entity(view_root.0).insert(RebuildView);
                                let mut context = RenderContext {
                                    commands: &mut commands,
                                    query_elements: &mut element_query,
                                    query_text: &mut text_query,
                                };
                                let out =
                                    context.render(&view_root.out, &asset.0, &view_root.props);

                                // If root changed, despawn old entities and replace with new.
                                if view_root.out != out {
                                    view_root.out.despawn_recursive(&mut commands);
                                    view_root.out = out;
                                }

                                // println!("create_views: {} {:?}", asset_path, ev);
                                // if let Some(ref template_node) = template.content {
                                //     let root = reconcile(
                                //         &mut commands,
                                //         &view_root.entities,
                                //         &template_node,
                                //         &mut element_query,
                                //         &mut text_query,
                                //         &server,
                                //         &assets,
                                //         &asset_path,
                                //         &view_root.props,
                                //     );
                                //     if view_root.entities != root {
                                //         view_root.entities = root;
                                //     }
                                // }
                            }
                        }

                        // Search for called template
                        // for mut view_element in root_query.iter_mut() {

                        // }
                    }

                    None => {
                        let status = server.load_state(*id);
                        if let Some(asset_path) = server.get_path(*id) {
                            warn!(
                                "Failure to load guise asset: {:?}, status [{:?}]",
                                asset_path, status
                            );
                        }
                    }
                }
            }

            AssetEvent::Removed { id } => {
                if let Some(asset_path) = server.get_path(*id) {
                    warn!("Guise asset Removed {:?}", asset_path);
                }
            }
        }
    }
}
