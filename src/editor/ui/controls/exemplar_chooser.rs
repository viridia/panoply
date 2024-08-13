use bevy::{prelude::*, ui};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::prelude::*;
use panoply_exemplar::{Exemplar, InstanceType};

/// View context component which stores the anchor element id for a menu.
#[derive(Component)]
struct SelectedExemplar(Option<AssetId<Exemplar>>);

#[derive(Clone, PartialEq)]
pub struct ExemplarChooser {
    pub selected: Option<AssetId<Exemplar>>,
    pub style: StyleHandle,
    pub on_change: Callback<Option<AssetId<Exemplar>>>,
    pub filter: String,
    pub instance_type: InstanceType,
}

impl ViewTemplate for ExemplarChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = self.on_change;
        cx.insert(SelectedExemplar(self.selected));
        let on_click =
            cx.create_callback(move |key: In<AssetId<Exemplar>>, mut commands: Commands| {
                commands.run_callback(on_change, Some(*key));
            });
        let asset_server = cx.use_resource_untracked::<AssetServer>();
        let exemplars = cx.use_resource_untracked::<Assets<Exemplar>>();

        let mut exemplars = exemplars
            .iter()
            .filter(|(_id, e)| e.0.meta_type == self.instance_type)
            .map(|(id, e)| {
                let path = asset_server.get_path(id).unwrap();
                ExemplarListItem {
                    id,
                    path: path.to_string(),
                    name: match e.0.display_name {
                        Some(ref name) => name.clone(),
                        None => path.label().unwrap_or("default").to_owned(),
                    },
                }
            })
            .collect::<Vec<_>>();
        exemplars.sort_by(|a, b| a.name.cmp(&b.name));
        // println!("Exemplars {}", exemplars.len());

        ListView::new()
            .style((style_list, self.style.clone()))
            .children((
                ListRow::new(())
                    .selected(self.selected.is_none())
                    .children("(erase)")
                    .on_click(
                        cx.create_callback(move |_: In<()>, mut commands: Commands| {
                            commands.run_callback(on_change, None);
                        }),
                    ),
                For::each_cmp(
                    exemplars,
                    |a, b| a.id == b.id,
                    move |loc| ExemplarRow {
                        key: loc.id,
                        name: loc.name.clone(),
                        on_click,
                    },
                ),
            ))
    }
}

fn style_list(ss: &mut StyleBuilder) {
    ss.min_height(ui::Val::Vh(80.)).flex_grow(1.);
}

#[derive(Clone, PartialEq)]
struct ExemplarListItem {
    path: String,
    id: AssetId<Exemplar>,
    name: String,
}

#[derive(Clone, PartialEq)]
struct ExemplarRow {
    key: AssetId<Exemplar>,
    name: String,
    on_click: Callback<AssetId<Exemplar>>,
}

impl ViewTemplate for ExemplarRow {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let selected = cx.use_inherited_component::<SelectedExemplar>().unwrap();
        ListRow::new(self.key)
            .selected(match (&self.key, selected.0.as_ref()) {
                (a, Some(b)) => *a == *b,
                _ => false,
            })
            .children(self.name.clone())
            .on_click(self.on_click)
    }
}
