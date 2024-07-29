use bevy::prelude::*;
use bevy_quill::prelude::*;
// use bevy_quill_obsidian::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeTerrainControls;

impl ViewTemplate for EditModeTerrainControls {
    type View = impl View;

    fn create(&self, _cx: &mut Cx) -> Self::View {
        Element::<NodeBundle>::new().children("Terrain Edit Controls")
    }
}
