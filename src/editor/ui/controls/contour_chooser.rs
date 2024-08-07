use bevy::{prelude::*, ui};
use bevy_mod_picking::{
    events::{Click, Pointer},
    prelude::On,
};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{colors, prelude::*, typography};

use crate::{
    editor::{
        events::{ChangeTerrainEvent, ThumbnailsReady},
        renderers::TerrainThumbnail,
        SelectedParcel,
    },
    terrain::{
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        Parcel, ShapeRef,
    },
};

fn style_listview(ss: &mut StyleBuilder) {
    ss.background_color(colors::U1)
        .border_radius(5.0)
        .padding(3);
}

fn style_listview_inner(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Flex)
        .flex_direction(ui::FlexDirection::Row)
        .flex_wrap(ui::FlexWrap::Wrap)
        .align_items(ui::AlignItems::Stretch)
        .align_self(ui::AlignSelf::Stretch)
        .justify_self(ui::JustifySelf::Stretch)
        .gap(2);
}

fn style_item(ss: &mut StyleBuilder) {
    ss.background_color(colors::U2).width(96).min_height(64);
}

fn style_item_id(ss: &mut StyleBuilder) {
    ss.position(ui::PositionType::Absolute)
        .right(4)
        .bottom(4)
        .font_size(11);
}

/// A scrollable list of items.
#[derive(Clone, PartialEq, Default)]
pub struct ContourChooser {
    /// Additional styles to be applied to the list view.
    pub style: StyleHandle,
}

impl ContourChooser {
    /// Create a new list view.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set additional styles to be applied to the list view.
    pub fn style<S: StyleTuple + 'static>(mut self, style: S) -> Self {
        self.style = style.into_handle();
        self
    }
}

impl ViewTemplate for ContourChooser {
    type View = ScrollView;
    fn create(&self, cx: &mut Cx) -> Self::View {
        let selected_parcel = cx.use_resource::<SelectedParcel>().0;
        let selected_contour = selected_parcel
            .and_then(|parcel| cx.use_component::<Parcel>(parcel))
            .map(|p| p.center_shape().shape as usize);

        let on_click: Callback<usize> = cx.create_callback(
            move |shape_id: In<usize>,
                  q_parcels: Query<&Parcel>,
                  mut commands: Commands,
                  r_selected: Res<SelectedParcel>| {
                let Some(parcel_id) = r_selected.0 else {
                    return;
                };
                let Ok(parcel) = q_parcels.get(parcel_id) else {
                    return;
                };
                commands.trigger(ChangeTerrainEvent {
                    realm: parcel.realm,
                    coords: parcel.coords,
                    shape: ShapeRef {
                        shape: *shape_id as u16,
                        rotation: parcel.center_shape().rotation,
                    },
                });
            },
        );

        let tc_handle = cx.use_resource::<TerrainContoursHandle>().0.clone();
        let tc_assets = cx.use_resource_untracked::<Assets<TerrainContoursTableAsset>>();

        let items = tc_assets
            .get(&tc_handle)
            .map(|contours| {
                let lock = contours.0.read().unwrap();
                let items = lock
                    .list()
                    .iter()
                    .map(|shape| ContourListEntry {
                        shape_id: shape.id,
                        // name: shape.name.clone(),
                        selected: Some(shape.id) == selected_contour,
                        on_click,
                    })
                    .collect::<Vec<_>>();
                // TODO: Sort
                items
            })
            .unwrap_or_default();

        ScrollView::new()
            .style((typography::text_default, style_listview, self.style.clone()))
            .content_style(style_listview_inner)
            .scroll_enable_y(true)
            .children(For::each(items, |item| ContourListEntry {
                shape_id: item.shape_id,
                selected: item.selected,
                on_click: item.on_click,
            }))
    }
}

#[derive(Clone, PartialEq, Component)]
struct ThumbnailRef(Option<Entity>);

#[derive(Clone, PartialEq)]
struct ContourListEntry {
    shape_id: usize,
    // name: String,
    selected: bool,
    on_click: Callback<usize>,
}

impl ViewTemplate for ContourListEntry {
    type View = impl View;
    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_click = self.on_click;
        let shape_id = self.shape_id;

        let owner = cx.owner();
        cx.create_effect(
            |world, owner| {
                let mut q_thumbnails = world.query::<(Entity, &TerrainThumbnail)>();
                let thumbnail_ent = q_thumbnails.iter(world).find_map(|(ent, thumb)| {
                    if thumb.contour_id == shape_id {
                        Some(ent)
                    } else {
                        None
                    }
                });

                world.entity_mut(owner).insert(ThumbnailRef(thumbnail_ent));
                world.entity_mut(owner).insert(Observer::new(
                    move |_ev: Trigger<ThumbnailsReady>,
                          mut commands: Commands,
                          q_thumbnails: Query<(Entity, &TerrainThumbnail)>| {
                        let thumbnail_ent = q_thumbnails.iter().find_map(|(ent, thumb)| {
                            if thumb.contour_id == shape_id {
                                Some(ent)
                            } else {
                                None
                            }
                        });

                        commands.entity(owner).insert(ThumbnailRef(thumbnail_ent));
                    },
                ));
            },
            owner,
        );

        let thumbnail_ent = cx.use_component::<ThumbnailRef>(owner).unwrap().0;
        let thumbnail_cmp =
            thumbnail_ent.map(|ent| cx.use_component::<TerrainThumbnail>(ent).unwrap());

        Element::<NodeBundle>::new()
            .style(style_item)
            .insert_dyn(
                move |_| {
                    On::<Pointer<Click>>::run(move |world: &mut World| {
                        // ev.stop_propagation();
                        world.run_callback(on_click, shape_id);
                    })
                },
                (),
            )
            .style_dyn(
                |selected, sb| {
                    if selected {
                        sb.background_color(colors::U3);
                    } else {
                        sb.background_color(colors::U2);
                    }
                },
                self.selected,
            )
            .style_dyn(
                |thumbnail, sb| {
                    if let Some(img) = thumbnail {
                        sb.background_image(img.clone());
                    }
                },
                thumbnail_cmp.map(|t| t.render_target.clone()),
            )
            .children((Element::<NodeBundle>::new()
                .style(style_item_id)
                .children(format!("{}", self.shape_id)),))
    }
}
