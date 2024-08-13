use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::prelude::*;

use crate::world::{WorldLocationsAsset, WorldLocationsResource};

/// View context component which stores the anchor element id for a menu.
#[derive(Component)]
struct SelectedLocation(Option<String>);

#[derive(Clone, PartialEq)]
pub struct LocationChooser {
    pub selected: Option<String>,
    pub on_change: Callback<String>,
}

impl ViewTemplate for LocationChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = self.on_change;
        cx.insert(SelectedLocation(self.selected.clone()));
        let on_click = cx.create_callback(move |key: In<String>, mut commands: Commands| {
            commands.run_callback(on_change, key.clone());
        });
        let locations_res = cx.use_resource::<WorldLocationsResource>().0.clone();
        let locations_asset = cx.use_resource_untracked::<Assets<WorldLocationsAsset>>();
        let mut locations: Vec<LocationListItem> = locations_asset
            .get(locations_res.id())
            .unwrap()
            .0
            .iter()
            .map(|loc| LocationListItem {
                name: loc.name.clone(),
                display_name: loc.name.to_owned(),
            })
            .collect();
        locations.sort_by(|a, b| a.name.cmp(&b.name));

        ListView::new().style(style_list).children(For::each_cmp(
            locations,
            |a, b| a.name == b.name,
            move |loc| LocationRow {
                key: loc.name.clone(),
                name: loc.display_name.clone(),
                on_click,
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
}

#[derive(Clone, PartialEq)]
struct LocationRow {
    key: String,
    name: String,
    on_click: Callback<String>,
}

impl ViewTemplate for LocationRow {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let selected = cx.use_inherited_component::<SelectedLocation>().unwrap();
        let on_click = self.on_click;
        ListRow::new(self.key.clone())
            .selected(match (&self.key, selected.0.as_ref()) {
                (a, Some(b)) => *a == *b,
                _ => false,
            })
            .children(self.name.clone())
            .on_click(on_click)
    }
}
