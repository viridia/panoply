use bevy::{prelude::*, ui::FocusPolicy};

use crate::guise::{
    asset::GuiseTemplatesLoader, controllers::SliderController, template::TemplateAsset, StyleAsset,
};

use super::{
    controller::Controller,
    controllers::{ButtonController, DefaultController},
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
        .add_asset_loader(GuiseTemplatesLoader)
        .add_asset::<StyleAsset>()
        .add_asset::<TemplateAsset>()
        .register_component_as::<dyn Controller, DefaultController>()
        .register_component_as::<dyn Controller, ButtonController>()
        .register_component_as::<dyn Controller, SliderController>()
        .register_type::<ButtonController>()
        .register_type::<SliderController>()
        .add_systems(Startup, create_test_ui)
        .add_systems(
            Update,
            ((
                create_views,
                attach_view_controllers,
                force_update,
                update_view_styles,
            )
                .chain(),),
        );
    }
}

fn create_test_ui(mut commands: Commands, server: Res<AssetServer>) {
    // let _handle: Handle<Template> = server.load("editor/ui/test.guise.json#templates/main");
    commands.spawn((
        ViewRoot {
            template: server.load("editor/ui/test.guise.json#templates/main"),
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

fn force_update(mut transforms: Query<&mut Transform>) {
    for mut transform in transforms.iter_mut() {
        transform.set_changed();
    }
}
