use bevy::{prelude::*, utils::HashMap};

use super::{
    view::{Cx, If, Sequence, TrackedResources},
    view_root::ViewRootResource,
    View, ViewRoot,
};

pub struct QuillPlugin;

impl Plugin for QuillPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_systems(Startup, create_test_ui)
            .add_systems(
                Update,
                (update_counter, render_views),
                // ((
                //     update_view_element_styles,
                //     force_update,
                // )
                //     .chain(),),
            )
            .init_resource::<Counter>()
            .insert_resource(ViewRootResource(ViewRoot::new(root_presenter, 1)));
    }
}

pub fn render_views(world: &mut World) {
    // for mut root in world.query::<&mut ViewRoot>().iter_mut(world) {
    //     // roots.push(root.handle.clone())
    //     root.build(world);
    // }
    world.resource_scope(|world, mut res: Mut<ViewRootResource>| {
        res.0.build(world);
        println!("Node count: {}", res.0.count());
    });
}

fn force_update(mut transforms: Query<&mut Transform>) {
    for mut transform in transforms.iter_mut() {
        transform.set_changed();
    }
}

fn root_presenter(mut cx: Cx<u8>) -> impl View {
    let mut counter = cx.use_resource_mut::<Counter>();
    counter.foo += 1;
    println!("{}", counter.foo);
    Sequence::new((
        "Root Presenter: ",
        format!("{}", counter.count),
        If::new(counter.count & 1 == 0, " [even]", " [odd]"),
    ))
}

#[derive(Resource, Default)]
pub struct Counter {
    pub count: u32,
    pub foo: usize,
}

fn update_counter(mut counter: ResMut<Counter>, key: Res<Input<KeyCode>>) {
    if key.pressed(KeyCode::Space) {
        counter.count += 1;
    }
}
