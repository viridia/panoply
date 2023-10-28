use bevy::prelude::*;

use super::{view_root::ViewRootResource, View};

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
                //     // attach_view_controllers,
                //     force_update,
                // )
                //     .chain(),),
            )
            .insert_resource(ViewRootResource::new(root_presenter));
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
    // let mut root = world.resource_mut::<ViewRootResource>();
    // root.build(world);
}

// fn create_test_ui(mut commands: Commands) {
//     commands.spawn(ViewRoot::new(root_presenter));
// }

fn force_update(mut transforms: Query<&mut Transform>) {
    for mut transform in transforms.iter_mut() {
        transform.set_changed();
    }
}

fn root_presenter() -> impl View {
    "Hello"
}
