use bevy::prelude::*;

use super::{
    asset::{ControlPanelResource, GuiseLoader},
    style::PartialStyle,
    template::Template,
};

pub struct GuisePlugin;

impl Plugin for GuisePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(GuiseLoader)
            .add_asset::<Template>()
            .add_asset::<PartialStyle>()
            .init_resource::<ControlPanelResource>();
    }
}
