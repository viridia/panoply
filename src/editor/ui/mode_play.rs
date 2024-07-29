use bevy::prelude::*;
use bevy_quill::prelude::*;
// use bevy_quill_obsidian::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct EditModePlayControls;

impl ViewTemplate for EditModePlayControls {
    type View = impl View;

    fn create(&self, _cx: &mut Cx) -> Self::View {
        Element::<NodeBundle>::new().children("Play Controls")
    }
}
