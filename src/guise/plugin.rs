use bevy::prelude::*;

use super::{
    asset::GuiseLoader,
    controllers::ButtonController,
    partial_style::PartialStyle,
    template::Template,
    view::{create_views, update_view_style_handles, update_view_styles, ViewRoot},
};

pub struct GuisePlugin;

impl Plugin for GuisePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(GuiseLoader)
            .add_asset::<Template>()
            .add_asset::<PartialStyle>()
            .register_type::<ButtonController>()
            .add_systems(Startup, create_test_ui)
            .add_systems(
                Update,
                ((
                    create_views,
                    apply_deferred,
                    update_view_styles,
                    apply_deferred,
                    update_view_style_handles,
                )
                    .chain(),),
            );
    }
}

fn create_test_ui(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        ViewRoot {
            template: server.load("editor/ui/test.guise.xml#main"),
        },
        // SpatialBundle::default(),
        NodeBundle {
            // background_color: Color::TURQUOISE.into(),
            // border_color: Color::YELLOW_GREEN.into(),
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                ..default()
            },
            ..default()
        },
    ));
    // let something = type_registry.0.read();
    // for _x in something.iter() {
    //     println!("Name {}", x.type_name());
    // }
}
