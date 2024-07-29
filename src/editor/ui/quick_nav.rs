use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{controls::Button, prelude::*};

use crate::{
    view::SetViewpointCmd,
    world::{WorldLocationsAsset, WorldLocationsResource},
};

use super::controls::LocationChooser;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct QuickNavOpen(pub bool);

#[derive(Clone, PartialEq)]
pub struct QuickNavDialog;

impl ViewTemplate for QuickNavDialog {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let selected = cx.create_mutable::<Option<String>>(None);
        let on_select = cx.create_callback(move |key: In<String>, world: &mut World| {
            selected.set_clone(world, Some(key.clone()));
        });
        let open = cx.use_resource::<State<QuickNavOpen>>();

        Dialog::new()
            .width(ui::Val::Px(400.))
            .open(open.0)
            .on_close(
                cx.create_callback(move |mut open: ResMut<NextState<QuickNavOpen>>| {
                    open.set(QuickNavOpen(false));
                }),
            )
            .children((
                DialogHeader::new().children("Quick Nav"),
                DialogBody::new().children(LocationChooser {
                    selected: selected.get_clone(cx),
                    on_change: on_select,
                }),
                DialogFooter::new().children((
                    Button::new()
                        .children("Cancel")
                        .on_click(cx.create_callback(
                            move |mut open: ResMut<NextState<QuickNavOpen>>| {
                                open.set(QuickNavOpen(false));
                            },
                        )),
                    Button::new()
                        .children("Go")
                        .variant(ButtonVariant::Primary)
                        .disabled(selected.as_ref(cx).is_none())
                        .autofocus(true)
                        .on_click(cx.create_callback(move |world: &mut World| {
                            let mut open =
                                world.get_resource_mut::<NextState<QuickNavOpen>>().unwrap();
                            open.set(QuickNavOpen(false));
                            let locations_res = world
                                .get_resource::<WorldLocationsResource>()
                                .unwrap()
                                .0
                                .clone();
                            let locations_asset =
                                world.get_resource::<Assets<WorldLocationsAsset>>().unwrap();
                            let location = match selected.get_clone(world) {
                                Some(name) => locations_asset
                                    .get(locations_res.id())
                                    .unwrap()
                                    .0
                                    .iter()
                                    .find(|loc| loc.name == name),
                                None => None,
                            };
                            match location {
                                Some(loc) => {
                                    let realm = loc.realm.clone();
                                    let position = loc.pos;
                                    world.commands().add(SetViewpointCmd { position, realm });
                                }
                                None => {
                                    println!("No location selected");
                                }
                            }
                        })),
                )),
            ))
    }
}
