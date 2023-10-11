use bevy::prelude::*;

use crate::guise::{
    asset::{GuiseAsset, GuiseAssetLoader},
    // controllers::SliderController,
    element::Element,
    element_style::ElementStyle,
    view_element::update_view_element_styles,
    // StyleAsset,
    view_root::render_views,
};

use super::ViewRoot;

// use super::{
//     controller::Controller,
//     controllers::{ButtonController, DefaultController},
//     view::*,
// };

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
            .init_asset_loader::<GuiseAssetLoader>()
            .init_asset::<GuiseAsset>()
            // .register_component_as::<dyn Controller, DefaultController>()
            // .register_component_as::<dyn Controller, ButtonController>()
            // .register_component_as::<dyn Controller, SliderController>()
            // .register_type::<ButtonController>()
            // .register_type::<SliderController>()
            .register_type::<Element>()
            .register_type::<ElementStyle>()
            .add_systems(Startup, create_test_ui)
            .add_systems(
                Update,
                ((
                    render_views,
                    update_view_element_styles,
                    // create_views,
                    // attach_view_controllers,
                    force_update,
                    // update_view_styles,
                )
                    .chain(),),
            );
    }
}

fn create_test_ui(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn(ViewRoot::new(server.load("editor/ui/test.guise#Main")));
}

fn force_update(mut transforms: Query<&mut Transform>) {
    for mut transform in transforms.iter_mut() {
        transform.set_changed();
    }
}
