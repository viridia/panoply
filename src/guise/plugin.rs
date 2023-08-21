use bevy::{prelude::*, ui::FocusPolicy};

use super::{
    asset::GuiseLoader,
    controller::Controller,
    controllers::{ButtonController, DefaultController},
    style::PartialStyle,
    template::Template,
    view::*,
};

pub struct GuisePlugin;

impl Plugin for GuisePlugin {
    fn build(&self, app: &mut App) {
        use bevy_trait_query::RegisterExt;
        app.add_plugins((
            bevy_mod_picking::picking_core::CorePlugin,
            bevy_mod_picking::picking_core::InteractionPlugin,
            bevy_mod_picking::input::InputPlugin,
            bevy_mod_picking::backends::bevy_ui::BevyUiBackend,
        ))
        .add_asset_loader(GuiseLoader)
        .add_asset::<Template>()
        .add_asset::<PartialStyle>()
        .register_component_as::<dyn Controller, DefaultController>()
        .register_component_as::<dyn Controller, ButtonController>()
        .register_type::<ButtonController>()
        .add_systems(Startup, create_test_ui)
        .add_systems(
            Update,
            ((
                create_views,
                attach_view_controllers,
                // apply_deferred,
                update_view_styles,
                // apply_deferred,
                update_view_styles_poll,
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
            focus_policy: FocusPolicy::Pass,
            ..default()
        },
    ));
}
