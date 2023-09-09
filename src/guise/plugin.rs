use bevy::prelude::*;

use crate::guise::{
    asset::{AssetSerial, GuiseTemplatesLoader},
    controllers::SliderController,
    template::TemplateAsset,
    StyleAsset,
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
        app
            // REINSTATE
            // .add_plugins((
            //     bevy_mod_picking::picking_core::CorePlugin,
            //     bevy_mod_picking::picking_core::InteractionPlugin,
            //     bevy_mod_picking::input::InputPlugin,
            //     bevy_mod_picking::backends::bevy_ui::BevyUiBackend,
            // ))
            .register_asset_loader(GuiseTemplatesLoader)
            .init_asset::<StyleAsset>()
            .init_asset::<TemplateAsset>()
            .init_asset::<AssetSerial>()
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
    commands.spawn(ViewRoot::new(
        server.load("editor/ui/test.guise.json#templates/main"),
    ));
}

fn force_update(mut transforms: Query<&mut Transform>) {
    for mut transform in transforms.iter_mut() {
        transform.set_changed();
    }
}
