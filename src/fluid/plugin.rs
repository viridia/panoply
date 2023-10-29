use bevy::prelude::*;

use super::{
    view::{Cx, Sequence},
    view_root::ViewRootResource,
    View,
};

pub struct FluidPlugin;

impl Plugin for FluidPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_systems(Startup, create_test_ui)
            .add_systems(
                Update,
                render_views,
                // ((
                //     update_view_element_styles,
                //     force_update,
                // )
                //     .chain(),),
            )
            .insert_resource(ViewRootResource::new(root_presenter, 1));
    }
}

pub fn render_views(world: &mut World) {
    // for mut root in world.query::<&mut ViewRoot>().iter_mut(world) {
    //     // roots.push(root.handle.clone())
    //     root.build(world);
    // }
    world.resource_scope(|world, mut res: Mut<ViewRootResource>| {
        res.build(world);
        println!("Node count: {}", res.count());
    });
}

fn force_update(mut transforms: Query<&mut Transform>) {
    for mut transform in transforms.iter_mut() {
        transform.set_changed();
    }
}

fn root_presenter(cx: Cx<u8>) -> impl View {
    Sequence::new(("Root Presenter: ", format!("{}", cx.props)))
}
