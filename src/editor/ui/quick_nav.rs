use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{controls::Button, prelude::*};

use crate::{
    view::SetViewpointCmd,
    world::{WorldLocationsAsset, WorldLocationsResource},
};

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

#[derive(Clone, PartialEq)]
pub struct LocationChooser {
    selected: Option<String>,
    on_change: Callback<String>,
}

impl ViewTemplate for LocationChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = self.on_change;
        let on_click = cx.create_callback(move |key: In<String>, world: &mut World| {
            world.run_callback(on_change, key.clone());
        });
        let locations_res = cx.use_resource::<WorldLocationsResource>().0.clone();
        let locations_asset = cx.use_resource_untracked::<Assets<WorldLocationsAsset>>();
        let selected = self.selected.clone();
        let mut locations: Vec<LocationListItem> = locations_asset
            .get(locations_res.id())
            .unwrap()
            .0
            .iter()
            .map(|loc| LocationListItem {
                name: loc.name.clone(),
                display_name: loc.display_name().to_owned(),
                selected: selected.as_ref().map_or(false, |s| *s == loc.name),
            })
            .collect();
        locations.sort_by(|a, b| a.name.cmp(&b.name));

        ListView::new().style(style_list).children(For::each_cmp(
            locations,
            |a, b| a.name == b.name && a.selected == b.selected,
            move |loc| {
                ListRow::new(loc.name.clone())
                    .selected(loc.selected)
                    .children(loc.display_name.clone())
                    .on_click(on_click)
            },
        ))
    }
}

fn style_list(ss: &mut StyleBuilder) {
    ss.min_height(ui::Val::Vh(80.)).flex_grow(1.);
}

#[derive(Clone, PartialEq)]
struct LocationListItem {
    name: String,
    display_name: String,
    selected: bool,
}
