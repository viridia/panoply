use bevy::prelude::*;
use bevy_quill::prelude::*;
// use bevy_quill_obsidian::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeMetadataControls;

impl ViewTemplate for EditModeMetadataControls {
    type View = impl View;

    fn create(&self, _cx: &mut Cx) -> Self::View {
        Element::<NodeBundle>::new().children("Metadata Edit Controls")
    }
}
