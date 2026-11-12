//! The property editor in scenery mode, for the exemplar chosen in the tool's list: a pane
//! captioned with its kind and name ("Fixture: WoodenChairSmall"), holding the exemplar's own
//! fields and then a subpane for each of its own aspects. Inherited aspects are not shown.

use std::{any::Any, sync::Arc};

use bevy::{
    annotations::{DisplayName, ReadOnly},
    app::PropagateOver,
    asset::AssetPath,
    ecs::{system::SystemParam, template::EntityTemplate},
    feathers::{
        constants::{fonts, size},
        containers::{pane, pane_body, pane_header},
        controls::{FeathersScrollbar, ScrollbarGutter},
        theme::ThemeTextColor,
    },
    text::FontSourceTemplate,
    prelude::*,
    reflect::TypeRegistry,
    ui_widgets::{ControlOrientation, NumericValue, ScrollArea},
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::text::ImmUiText,
    Imm,
};
use panoply_exemplar::Exemplar;

use crate::{
    exemplar_list::{chosen_exemplar, SceneryChoices},
    mode_selector::{EditorModeSettings, SceneryTool},
    properties::{
        edit::{
            apply_property_edits, count_edit_gestures, read_inspected_values, EditGesture,
            ExemplarAspect, InspectedRoots, InspectedValues, PropertyEdits, PropertyRoot,
        },
        root_pane, root_rows, CapsPanel, HEADER_TEXT,
    },
};

/// The immediate-mode root of scenery mode's property editor.
#[derive(Component, Default, Clone)]
pub(crate) struct SceneryProperties;

pub struct SceneryPropertiesPlugin;

impl Plugin for SceneryPropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyImmediateAttachPlugin::<CapsPanel, SceneryProperties>::new())
            .init_resource::<PropertyEdits>()
            .init_resource::<EditGesture>()
            .init_resource::<InspectedRoots>()
            .init_resource::<InspectedValues>()
            // So plans can read a number input's value.
            .register_type::<NumericValue>()
            .register_type_data::<NumericValue, ReflectComponent>()
            .add_systems(PreUpdate, count_edit_gestures)
            // After the panel has run in `Update`: its edits are made, then its values copied
            // for the next frame.
            .add_systems(
                PostUpdate,
                (
                    apply_property_edits,
                    inspect_chosen_exemplar,
                    read_inspected_values,
                )
                    .chain(),
            );
    }
}

/// An exemplar's own fields, as its pane shows them: `ExemplarData` is not reflected.
#[derive(Reflect, Clone, Debug)]
struct ExemplarFields {
    #[reflect(@ReadOnly)]
    extends: Option<String>,
    #[reflect(@ReadOnly, @DisplayName("display name"))]
    display_name: Option<String>,
    #[reflect(@ReadOnly, @DisplayName("abstract"))]
    is_abstract: bool,
}

/// The exemplar itself, the panel's first pane, captioned with its kind and name.
struct ExemplarRoot {
    exemplar: AssetPath<'static>,
    kind: &'static str,
    name: String,
}

impl PropertyRoot for ExemplarRoot {
    fn caption(&self) -> String {
        format!("{}: {}", self.kind, self.name)
    }

    fn key(&self) -> String {
        self.exemplar.to_string()
    }

    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let server = world.resource::<AssetServer>();
        let id = server
            .get_path_id(&self.exemplar)?
            .try_typed::<Exemplar>()
            .ok()?;
        let data = &world.resource::<Assets<Exemplar>>().get(id)?.0;
        Some(Box::new(ExemplarFields {
            extends: data
                .extends
                .as_ref()
                .and_then(|handle| server.get_path(handle))
                .map(|path| path.to_string()),
            display_name: data.display_name.clone(),
            is_abstract: data.is_abstract,
        }))
    }

    fn modify(
        &self,
        _world: &mut World,
        _change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        // TODO(PAN28d): the exemplar's own fields become editable.
        Err("the exemplar's own fields cannot be edited".into())
    }
}

/// The kind of exemplar `tool` places, as the panel's heading names it.
fn kind(tool: SceneryTool) -> &'static str {
    match tool {
        SceneryTool::PlaceWall => "Wall",
        SceneryTool::PlaceFixture => "Fixture",
        SceneryTool::PlaceActor => "Actor",
        SceneryTool::DrawTerrainFx => "Terrain effect",
        _ => "",
    }
}

/// Inspects the own aspects of the exemplar chosen in the current tool's list, if any.
fn inspect_chosen_exemplar(
    settings: Res<EditorModeSettings>,
    choices: Res<SceneryChoices>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut inspected: ResMut<InspectedRoots>,
) {
    let tool = settings.scenery_tool;
    let chosen = choices.choice(tool).and_then(|name| {
        let id = chosen_exemplar(&server, &exemplars, tool, &name)?;
        Some((name, id))
    });
    let Some((name, id)) = chosen else {
        if !inspected.roots.is_empty() {
            inspected.roots.clear();
        }
        return;
    };
    let (Some(path), Some(exemplar)) = (server.get_path(id), exemplars.get(id)) else {
        return;
    };
    let mut roots: Vec<Arc<dyn PropertyRoot>> = vec![Arc::new(ExemplarRoot {
        exemplar: path.clone_owned(),
        kind: kind(tool),
        name,
    })];
    roots.extend(exemplar.0.aspects.iter().map(|aspect| {
        Arc::new(ExemplarAspect {
            exemplar: path.clone_owned(),
            aspect: aspect.reflect_short_type_path().to_string(),
        }) as Arc<dyn PropertyRoot>
    }));
    let same = inspected.roots.len() == roots.len()
        && inspected
            .roots
            .iter()
            .zip(&roots)
            .all(|(old, new)| old.key() == new.key() && old.caption() == new.caption());
    if !same {
        inspected.roots = roots;
    }
}

#[derive(SystemParam)]
pub(crate) struct SceneryPropertiesParams<'w> {
    inspected: Res<'w, InspectedValues>,
    edits: ResMut<'w, PropertyEdits>,
    registry: Res<'w, AppTypeRegistry>,
}

impl ImmediateAttach<CapsPanel> for SceneryProperties {
    type Params = SceneryPropertiesParams<'static>;

    /// A pane for the exemplar, the first value, filling the panel's last row: its header fixed,
    /// and its body scrolling, holding the exemplar's own fields' rows, then a subpane per aspect,
    /// the rest. The exemplar holds its aspects.
    fn construct(ui: &mut Imm<CapsPanel>, params: &mut SceneryPropertiesParams) {
        let edits = params.edits.as_mut();
        let registry = params.registry.read();
        let Some(((exemplar, fields), aspects)) = params.inspected.values.split_first() else {
            return;
        };
        let Some(root) = (exemplar.as_ref() as &dyn Any).downcast_ref::<ExemplarRoot>() else {
            return;
        };
        let kind = format!("{}:", root.kind);
        let name = &root.name;
        ui.ch_id("pane")
            .on_spawn_apply_scene(|| bsn! { @pane() Node { flex_grow: 1.0, min_height: px(0) } })
            .add(|ui| {
                // "Fixture: WoodenChairSmall", the name in bold.
                ui.ch().on_spawn_apply_scene(pane_header).add(|ui| {
                    ui.ch()
                        .on_spawn_insert(|| Node {
                            column_gap: px(4),
                            align_items: AlignItems::Baseline,
                            ..default()
                        })
                        .add(|ui| {
                            ui.ch()
                                .on_spawn_apply_scene(|| header_text(fonts::REGULAR, FontWeight::NORMAL))
                                .on_hash_change_text_fn(&kind, || kind.clone());
                            ui.ch()
                                .on_spawn_apply_scene(|| header_text(fonts::BOLD, FontWeight::BOLD))
                                .on_hash_change_text_fn(name, || name.clone());
                        });
                });
                ui.ch()
                    .on_spawn_apply_scene(|| {
                        bsn! { @pane_body() Node { flex_grow: 1.0, min_height: px(0) } }
                    })
                    .add(|ui| {
                        // The scrollbar sits in this frame's right padding, its gutter, which
                        // the scrollbar sets as it shows and hides: the body's own padding is
                        // left alone.
                        ui.ch()
                            .on_spawn_insert(|| {
                                (
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        min_height: px(0),
                                        ..default()
                                    },
                                    ScrollbarGutter(px(14)),
                                )
                            })
                            .add(|ui| {
                                scrolled_rows(
                                    ui,
                                    exemplar,
                                    fields.as_ref(),
                                    aspects,
                                    &registry,
                                    edits,
                                )
                            });
                    });
            });
    }
}

/// Part of the pane's caption, in `font` at the header's size: the header's own font does not
/// reach text inside a nested node.
fn header_text(font: &'static str, weight: FontWeight) -> impl Scene {
    bsn! {
        Text("")
        TextFont {
            font: FontSourceTemplate::Handle(font),
            font_size: size::MEDIUM_FONT,
            weight: weight,
        }
        PropagateOver<TextFont>
        ThemeTextColor(HEADER_TEXT)
    }
}

/// The exemplar's own fields' rows, then a subpane per aspect, in a column that scrolls, with
/// its scrollbar.
fn scrolled_rows(
    ui: &mut Imm<CapsPanel>,
    exemplar: &Arc<dyn PropertyRoot>,
    fields: &dyn PartialReflect,
    aspects: &[(Arc<dyn PropertyRoot>, Box<dyn PartialReflect>)],
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
) {
    let rows = ui
        .ch_id("rows")
        .on_spawn_insert(|| {
            (
                Name::new("Rows"),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    flex_grow: 1.0,
                    min_height: px(0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollArea,
            )
        })
        .add(|ui| {
            root_rows(ui, exemplar, fields, registry, edits);
            for (root, value) in aspects {
                root_pane(ui, root, value.as_ref(), registry, edits);
            }
        });
    let target = rows.entity();
    ui.ch_id("scrollbar").on_spawn_apply_scene(move || {
        bsn! {
            @FeathersScrollbar {
                @target: {EntityTemplate::Entity(target)},
                @orientation: {ControlOrientation::Vertical}
            }
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: px(6),
            }
        }
    });
}
