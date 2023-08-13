use bevy::prelude::*;

use super::{
    asset::GuiseLoader,
    controllers::ButtonController,
    style::PartialStyle,
    template::Template,
    view::{create_views, update_view_styles, ViewRoot},
};

pub struct GuisePlugin;

impl Plugin for GuisePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(GuiseLoader)
            .add_asset::<Template>()
            .add_asset::<PartialStyle>()
            .register_type::<ButtonController>()
            .add_systems(Startup, create_test_ui)
            .add_systems(Update, (create_views, update_view_styles));
    }
}

fn create_test_ui(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn(ViewRoot {
        template: server.load("editor/ui/test.guise.xml#main"),
    });
    // let something = type_registry.0.read();
    // for _x in something.iter() {
    //     println!("Name {}", x.type_name());
    // }
}
