//! The property editor in scenery mode, for the exemplar chosen in the tool's list: a pane
//! captioned with its kind and name ("Fixture: WoodenChairSmall"), holding the exemplar's own
//! fields and then a subpane for each of its own aspects, each with a × that removes it. The
//! caption's menu adds the exemplar's absent fields and opens the aspects chooser
//! (`aspect_chooser`). Inherited aspects are not shown.

use std::{any::Any, sync::Arc};

use bevy::{
    annotations::DisplayName,
    app::PropagateOver,
    asset::{AssetPath, UntypedAssetId},
    ecs::{system::SystemParam, template::EntityTemplate},
    feathers::{
        constants::{fonts, size},
        containers::{pane, pane_body, pane_header},
        controls::{FeathersScrollbar, ScrollbarGutter},
        theme::ThemeTextColor,
    },
    platform::collections::HashMap,
    text::FontSourceTemplate,
    prelude::*,
    reflect::{ParsedPath, TypeRegistry},
    ui_widgets::{ControlOrientation, NumericValue, ScrollArea},
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::text::ImmUiText,
    Imm,
};
use panoply_exemplar::{Aspect, Exemplar, ExemplarData};

use crate::{
    aspect_chooser::open_aspect_chooser,
    chooser::{chooser_dialog, ItemChosen},
    exemplar_list::{chosen_exemplar, named_exemplars, SceneryChoices},
    mode_selector::{EditorModeSettings, SceneryTool},
    properties::{
        edit::{
            apply_property_edits, count_edit_gestures, read_inspected_values, AspectHolder,
            EditGesture, ExemplarAspect, InspectedRoots, InspectedValues, PathSteps,
            PropertyEdits, PropertyRoot, ReflectAspectHolder,
        },
        root_add_menu, root_pane, root_rows, CapsPanel, HEADER_TEXT,
    },
    unsaved::UnsavedAssets,
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
            // So an edit can find what each root can do (`AspectHolder`).
            .register_type::<ExemplarRoot>()
            .register_type::<ExemplarAspect>()
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
    extends: ExemplarRef,
    #[reflect(@DisplayName("display name"))]
    display_name: Option<String>,
    #[reflect(@DisplayName("abstract"))]
    is_abstract: bool,
}

impl ExemplarFields {
    fn of(data: &ExemplarData, server: &AssetServer) -> Self {
        Self {
            extends: ExemplarRef(
                data.extends
                    .as_ref()
                    .and_then(|handle| server.get_path(handle))
                    .map(|path| path.to_string()),
            ),
            display_name: data.display_name.clone(),
            is_abstract: data.is_abstract,
        }
    }
}

/// The exemplar another extends, by its path, or none. Always a row in the panel, unlike an
/// optional field: a button naming the prototype, which opens a chooser of them, and a × that
/// clears it.
#[derive(Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Default)]
pub(crate) struct ExemplarRef(pub Option<String>);

/// The exemplar itself, the panel's first pane, captioned with its kind and name.
#[derive(Reflect)]
#[reflect(AspectHolder)]
pub(crate) struct ExemplarRoot {
    exemplar: AssetPath<'static>,
    kind: &'static str,
    name: String,
}

impl ExemplarRoot {
    /// The exemplar, and the catalog where it is kept.
    fn ids(&self, server: &AssetServer) -> Result<(AssetId<Exemplar>, UntypedAssetId), BevyError> {
        let id = server
            .get_path_id(&self.exemplar)
            .and_then(|id| id.try_typed::<Exemplar>().ok())
            .ok_or_else(|| format!("{} is not loaded", self.exemplar))?;
        let catalog = server
            .get_path_id(self.exemplar.without_label())
            .ok_or_else(|| format!("{} has no catalog", self.exemplar))?;
        Ok((id, catalog))
    }

    /// Changes the exemplar's data through `change`, marking its catalog unsaved.
    fn edit(
        &self,
        world: &mut World,
        change: impl FnOnce(&mut ExemplarData, &AssetServer) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        let server = world.resource::<AssetServer>().clone();
        let (id, catalog) = self.ids(&server)?;
        world.resource_scope(|_, mut exemplars: Mut<Assets<Exemplar>>| {
            Exemplar::edit(&mut exemplars, id, |data| change(data, &server))
        })?;
        world.resource_mut::<UnsavedAssets>().mark(catalog);
        Ok(())
    }
}

impl PropertyRoot for ExemplarRoot {
    /// The name alone, for the undo history ("Set Marcus.display_name"): the pane's header puts
    /// the kind before it.
    fn caption(&self) -> String {
        self.name.clone()
    }

    fn key(&self) -> String {
        self.exemplar.to_string()
    }

    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let (id, _) = self.ids(world.resource::<AssetServer>()).ok()?;
        let data = &world.resource::<Assets<Exemplar>>().get(id)?.0;
        Some(Box::new(ExemplarFields::of(
            data,
            world.resource::<AssetServer>(),
        )))
    }

    fn modify(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        self.edit(world, |data, server| {
            let mut fields = ExemplarFields::of(data, server);
            change(&mut fields)?;
            data.display_name = fields.display_name;
            data.is_abstract = fields.is_abstract;
            data.extends = fields.extends.0.map(|path| server.load::<Exemplar>(path));
            Ok(())
        })
    }
}

impl AspectHolder for ExemplarRoot {
    fn modify_aspects(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut Vec<Box<dyn Aspect>>) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        self.edit(world, |data, _| change(&mut data.aspects))
    }
}

/// The exemplar `root` is, if it is an [`ExemplarRoot`].
pub(crate) fn exemplar_of(root: &dyn PropertyRoot, server: &AssetServer) -> Option<AssetId<Exemplar>> {
    let exemplar = (root as &dyn Any).downcast_ref::<ExemplarRoot>()?;
    exemplar.ids(server).ok().map(|(id, _)| id)
}

/// Opens a chooser of the exemplars `root`, an [`ExemplarRoot`], may extend: those of its type,
/// abstract ones included, leaving out itself and any that extend it, which would make its chain
/// a cycle. The one chosen is set as its `extends`.
pub(crate) fn open_extends_chooser(
    In(root): In<Arc<dyn PropertyRoot>>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut commands: Commands,
) {
    let Some(exemplar) = (root.as_ref() as &dyn Any).downcast_ref::<ExemplarRoot>() else {
        return;
    };
    let Ok((this, _)) = exemplar.ids(&server) else {
        return;
    };
    let Some(meta_type) = exemplars.get(this).map(|data| data.0.meta_type) else {
        return;
    };
    let paths: HashMap<String, String> = named_exemplars(&server, &exemplars, meta_type)
        .into_iter()
        .filter(|(_, candidate)| !Exemplar::chain_contains(&exemplars, *candidate, this))
        .filter_map(|(name, candidate)| Some((name, server.get_path(candidate)?.to_string())))
        .collect();
    let names = paths.keys().cloned().collect();
    commands
        .spawn_scene(chooser_dialog("Extends", "Choose", names))
        .insert(Name::new("ExtendsChooser"))
        .observe(move |chosen: On<ItemChosen>, mut edits: ResMut<PropertyEdits>| {
            if let Some(path) = paths.get(&chosen.name) {
                let field = ParsedPath::empty().with_field("extends");
                edits.set(&root, field, ExemplarRef(Some(path.clone())));
            }
        });
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
                // "Fixture: WoodenChairSmall", the name in bold, then the menu that adds its
                // absent fields and opens the aspects chooser.
                ui.ch()
                    .on_spawn_apply_scene(pane_header)
                    .on_spawn_insert(|| Name::new("ExemplarHeader"))
                    .add(|ui| {
                        ui.ch()
                            .on_spawn_insert(|| Node {
                                column_gap: px(4),
                                align_items: AlignItems::Baseline,
                                ..default()
                            })
                            .add(|ui| {
                                ui.ch()
                                    .on_spawn_apply_scene(|| {
                                        header_text(fonts::REGULAR, FontWeight::NORMAL)
                                    })
                                    .on_hash_change_text_fn(&kind, || kind.clone());
                                ui.ch()
                                    .on_spawn_apply_scene(|| header_text(fonts::BOLD, FontWeight::BOLD))
                                    .on_hash_change_text_fn(name, || name.clone());
                            });
                        let more = ("aspects", "Aspects…");
                        if root_add_menu(ui, exemplar, fields.as_ref(), &registry, edits, more) {
                            let root = exemplar.clone();
                            ui.commands_mut()
                                .run_system_cached_with(open_aspect_chooser, root);
                        }
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
                let mut remove = false;
                root_pane(ui, root, value.as_ref(), registry, edits, Some(&mut remove));
                if remove {
                    edits.change_aspects(exemplar, Vec::new(), vec![root.caption()]);
                }
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
