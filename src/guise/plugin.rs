use bevy::{prelude::*, ui::FocusPolicy};
use bevy_mod_picking::{prelude::*, DefaultPickingPlugins};

use super::{
    asset::GuiseLoader,
    controllers::{init_button, ButtonController},
    style::PartialStyle,
    template::Template,
    view::{create_views, update_view_style_handles, update_view_styles, ViewRoot},
};

pub struct GuisePlugin;

impl Plugin for GuisePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_asset_loader(GuiseLoader)
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
                    init_button,
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
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                ..default()
            },
            focus_policy: FocusPolicy::Block,
            ..default()
        },
        On::<Pointer<Over>>::run(|| println!("Over!")),
        On::<Pointer<Move>>::run(|| println!("Move!")),
    ));
}
