use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::prelude::*;

use crate::world::{WorldLocationsAsset, WorldLocationsResource};

#[derive(Clone, PartialEq)]
pub struct LocationChooser {
    pub selected: Option<String>,
    pub on_change: Callback<String>,
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
                display_name: loc.name.to_owned(),
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
