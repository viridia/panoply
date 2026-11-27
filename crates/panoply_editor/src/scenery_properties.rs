//! The property editor in scenery mode, for the exemplar chosen in the tool's list, or in
//! selection mode the selected instance: a pane captioned with its kind and name ("Fixture:
//! WoodenChairSmall"), holding the root's own fields (an exemplar's, or an instance's label) and
//! then a subpane per aspect the root resolves to, own or inherited. The caption's menu adds the
//! root's absent fields, and a button beside it opens the aspects chooser (`aspect_chooser`), as
//! Blender's Add Modifier does.
//!
//! The aspects are in the order of the root's exemplar chain, from its farthest exemplar to the
//! root, each where it first appears, then those the root adds that nothing along the chain has
//! ([`aspect_roots`]). An inherited aspect is shown disabled until its pencil overrides it, which
//! copies it into the root's own aspects, in the same place; an own aspect's × takes it out, so
//! the inherited one shows again where it overrode one. After an aspect come those it requires that
//! the root's resolved aspects lack, each a header with a + that adds it (D121).

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
        tokens,
    },
    platform::collections::HashMap,
    prelude::*,
    reflect::{ParsedPath, TypeRegistry},
    text::FontSourceTemplate,
    ui_widgets::{ControlOrientation, NumericValue, ScrollArea},
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::text::ImmUiText,
    Imm,
};
use panoply_exemplar::{
    aspect_mut, required_aspect, Aspect, Exemplar, ExemplarData, InstanceAspects, InstanceKind,
    ScriptedAspect,
};
use panoply_precinct::PrecinctAsset;
use panoply_scenery::precinct::Precinct;

use crate::{
    aspect_chooser::open_aspect_chooser,
    chooser::{chooser_dialog, ItemChosen},
    exemplar_list::{chosen_exemplar, named_exemplars, placed_entry, SceneryChoices},
    instance_selection::ScenerySelection,
    mode_selector::{EditorModeSettings, SceneryTool},
    precinct_edit::EditedPrecincts,
    properties::{
        disable_inherited_rows,
        edit::{
            apply_property_edits, as_aspect, aspect_name, count_edit_gestures, pane_role,
            read_inspected_values, AspectHolder, AspectPane, EditGesture, EditKind, ExemplarAspect,
            ExemplarPart, InheritedAspect, InspectedRoots, InspectedValue, InspectedValues,
            MissingAspect, PaneRole, PathSteps, PropertyEdits, PropertyRoot, ReflectAspectHolder,
            ReflectAspectPane, ReflectExemplarPart, RequiredDecls,
        },
        icon_button, root_add_menu, root_pane, root_rows, CapsPanel, CollapsedRows, PaneAction,
        HEADER_TEXT,
    },
    unsaved::UnsavedAssets,
};

/// The icon of the root pane's button that opens the aspects chooser.
const ASPECTS_ICON: &str = "editor/icons/lucide/blocks.png";

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
            .init_resource::<CollapsedRows>()
            // So an edit can find what each root can do (`AspectHolder`, `AspectPane`).
            .register_type::<ExemplarRoot>()
            .register_type::<ExemplarAspect>()
            .register_type::<InheritedAspect>()
            .register_type::<MissingAspect>()
            .register_type::<InstanceRoot>()
            .register_type::<InstanceAspect>()
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
                    inspect_scenery_roots,
                    read_inspected_values,
                    disable_inherited_rows,
                )
                    .chain(),
            );
    }
}

/// An exemplar's own fields, as its pane shows them: `ExemplarData` is not reflected.
#[derive(Reflect, Clone, Debug)]
struct ExemplarFields {
    #[reflect(@DisplayName("Extends"))]
    extends: ExemplarRef,
    #[reflect(@DisplayName("Display name"))]
    display_name: Option<String>,
    #[reflect(@DisplayName("Abstract"))]
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

    fn kind(&self) -> String {
        self.kind.to_string()
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
        change: &mut dyn FnMut(&mut Vec<Arc<dyn Aspect>>) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        self.edit(world, |data, _| change(&mut data.aspects))
    }
}

/// Which of a precinct's lists holds an instance.
#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstanceList {
    Scenery,
    Actor,
}

/// An instance's label, set through Rename…'s dialog rather than typed in place, whose row shows it
/// beside the button.
#[derive(Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Default)]
pub(crate) struct InstanceLabel(pub Option<String>);

/// An instance's own fields, as its pane shows them.
#[derive(Reflect, Clone, Debug)]
struct InstanceFields {
    #[reflect(@DisplayName("Label"))]
    label: InstanceLabel,
}

/// An instance placed by a precinct, the panel's first pane in selection mode, captioned with its
/// kind and its exemplar's name. It is named by its precinct's asset path and its place in one of
/// the precinct's lists, not by its id: a save renumbers internal ids as the precinct reloads
/// (D87), but keeps the lists' order, and an undo finds the lists as they were after its edit.
#[derive(Reflect, Clone, Debug)]
#[reflect(AspectHolder, ExemplarPart)]
pub(crate) struct InstanceRoot {
    precinct: AssetPath<'static>,
    list: InstanceList,
    index: usize,
    /// Its exemplar, a `file#Label` path, which an edit checks the instance still has.
    exemplar: String,
    kind: String,
    name: String,
}

impl InstanceRoot {
    /// The precinct's asset.
    fn asset_id(&self, server: &AssetServer) -> Result<AssetId<PrecinctAsset>, BevyError> {
        server
            .get_path_id(&self.precinct)
            .and_then(|id| id.try_typed::<PrecinctAsset>().ok())
            .ok_or_else(|| format!("{} is not loaded", self.precinct).into())
    }

    /// The instance's own aspects and label in `asset`, if it is still there.
    fn find<'a>(&self, asset: &'a PrecinctAsset) -> Option<(&'a InstanceAspects, Option<&'a str>)> {
        match self.list {
            InstanceList::Scenery => {
                let instance = asset.scenery.get(self.index)?;
                let exemplar = asset.scenery_types.get(instance.id)?;
                (*exemplar == self.exemplar)
                    .then_some((&instance.aspects, instance.label.as_deref()))
            }
            InstanceList::Actor => {
                let actor = asset.actors.get(self.index)?;
                (actor.exemplar == self.exemplar)
                    .then_some((&actor.aspects, actor.label.as_deref()))
            }
        }
    }

    /// The instance's own aspects in `asset`, to change.
    fn aspects_mut<'a>(&self, asset: &'a mut PrecinctAsset) -> Option<&'a mut InstanceAspects> {
        match self.list {
            InstanceList::Scenery => {
                let instance = asset.scenery.get_mut(self.index)?;
                let exemplar = asset.scenery_types.get(instance.id)?;
                (*exemplar == self.exemplar).then_some(&mut instance.aspects)
            }
            InstanceList::Actor => {
                let actor = asset.actors.get_mut(self.index)?;
                (actor.exemplar == self.exemplar).then_some(&mut actor.aspects)
            }
        }
    }
}

impl PropertyRoot for InstanceRoot {
    /// Its exemplar's name, for the undo history ("Override WoodenChest.Lock"): the pane's header
    /// puts the kind before it.
    fn caption(&self) -> String {
        self.name.clone()
    }

    fn kind(&self) -> String {
        self.kind.clone()
    }

    fn key(&self) -> String {
        format!("{} {:?} {}", self.precinct, self.list, self.index)
    }

    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let id = self.asset_id(world.resource::<AssetServer>()).ok()?;
        let asset = world.resource::<Assets<PrecinctAsset>>().get(id)?;
        let (_, label) = self.find(asset)?;
        Some(Box::new(InstanceFields {
            label: InstanceLabel(label.map(str::to_string)),
        }))
    }

    /// Refused: the label, its one field, is set by its dialog, an edit of its precinct.
    fn modify(
        &self,
        _world: &mut World,
        _change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        Err("an instance's label is set by Rename…".into())
    }
}

impl AspectHolder for InstanceRoot {
    /// Changes a copy of the instance's own aspects, and keeps it only if `change` succeeds,
    /// marking the precinct unsaved and holding it loaded (`precinct_edit`).
    fn modify_aspects(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut Vec<Arc<dyn Aspect>>) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        let server = world.resource::<AssetServer>().clone();
        let id = self.asset_id(&server)?;
        let gone = || {
            format!(
                "{} has no {} at {}",
                self.precinct, self.exemplar, self.index
            )
        };
        let mut aspects = {
            let asset = world
                .resource::<Assets<PrecinctAsset>>()
                .get(id)
                .ok_or_else(|| format!("{} is not loaded", self.precinct))?;
            self.find(asset).ok_or_else(gone)?.0.clone()
        };
        change(&mut aspects.0)?;
        {
            let mut assets = world.resource_mut::<Assets<PrecinctAsset>>();
            let mut asset = assets.get_mut(id).ok_or("the precinct is gone")?;
            *self.aspects_mut(&mut asset).ok_or_else(gone)? = aspects;
        }
        world.resource_mut::<UnsavedAssets>().mark(id);
        if let Some(handle) = server.get_id_handle(id) {
            world.resource_mut::<EditedPrecincts>().hold(handle);
        }
        Ok(())
    }
}

impl ExemplarPart for InstanceRoot {
    fn exemplar_id(&self, world: &World) -> Option<AssetId<Exemplar>> {
        world
            .resource::<AssetServer>()
            .get_path_id(&self.exemplar)?
            .try_typed::<Exemplar>()
            .ok()
    }
}

/// An aspect of an instance, its own.
#[derive(Reflect, Clone, Debug)]
#[reflect(ExemplarPart, AspectPane)]
pub(crate) struct InstanceAspect {
    instance: InstanceRoot,
    aspect: String,
    /// Whether the instance's exemplar chain has the aspect too.
    overrides: bool,
}

impl PropertyRoot for InstanceAspect {
    fn caption(&self) -> String {
        aspect_name(&self.aspect).to_string()
    }

    fn key(&self) -> String {
        format!("{} {}", self.instance.key(), self.aspect)
    }

    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let id = self
            .instance
            .asset_id(world.resource::<AssetServer>())
            .ok()?;
        let asset = world.resource::<Assets<PrecinctAsset>>().get(id)?;
        let (aspects, _) = self.instance.find(asset)?;
        let aspect = aspects
            .0
            .iter()
            .find(|aspect| aspect.key() == self.aspect)?;
        Some(aspect.clone_boxed().into_partial_reflect())
    }

    fn modify(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        self.instance.modify_aspects(world, &mut |aspects| {
            let aspect = aspects
                .iter_mut()
                .find(|aspect| aspect.key() == self.aspect)
                .ok_or_else(|| format!("the instance has no {}", self.aspect))?;
            change(aspect_mut(aspect).as_partial_reflect_mut())
        })
    }
}

impl ExemplarPart for InstanceAspect {
    fn exemplar_id(&self, world: &World) -> Option<AssetId<Exemplar>> {
        self.instance.exemplar_id(world)
    }
}

impl AspectPane for InstanceAspect {
    fn aspect_key(&self) -> &str {
        &self.aspect
    }

    fn role(&self) -> PaneRole {
        PaneRole::Own {
            overrides: self.overrides,
        }
    }
}

/// The exemplar `root` is, if it is an [`ExemplarRoot`].
pub(crate) fn exemplar_of(
    root: &dyn PropertyRoot,
    server: &AssetServer,
) -> Option<AssetId<Exemplar>> {
    let exemplar = (root as &dyn Any).downcast_ref::<ExemplarRoot>()?;
    exemplar.ids(server).ok().map(|(id, _)| id)
}

/// What the aspects chooser lists for a root that holds aspects.
pub(crate) struct HeldAspects {
    /// What the root's instances are, which an aspect must attach to.
    pub kind: Option<InstanceKind>,
    /// The keys of the root's own aspects.
    pub own: Vec<String>,
    /// The keys of the aspects it inherits, which the chooser leaves out: the pencil overrides
    /// them.
    pub inherited: Vec<String>,
    /// Whether the root is an instance, which takes native aspects only, since a precinct does not
    /// load scripted ones.
    // TODO(PAN135): scripted aspects on instances.
    pub instance: bool,
}

/// The aspects `root`, an exemplar's or an instance's root, holds and inherits.
pub(crate) fn held_aspects(root: &dyn PropertyRoot, world: &World) -> Option<HeldAspects> {
    let server = world.resource::<AssetServer>();
    let exemplars = world.resource::<Assets<Exemplar>>();
    let keys = |aspects: &[Arc<dyn Aspect>]| -> Vec<String> {
        aspects
            .iter()
            .map(|aspect| aspect.key().to_string())
            .collect()
    };
    let inherited = |chain: &[(AssetId<Exemplar>, &ExemplarData)]| -> Vec<String> {
        chain
            .iter()
            .flat_map(|(_, data)| keys(&data.aspects))
            .collect()
    };
    if let Some(id) = exemplar_of(root, server) {
        let chain = Exemplar::chain(exemplars, id);
        let (_, data) = chain.first()?;
        return Some(HeldAspects {
            kind: data.kind,
            own: keys(&data.aspects),
            inherited: inherited(&chain[1..]),
            instance: false,
        });
    }
    let instance = (root as &dyn Any).downcast_ref::<InstanceRoot>()?;
    let asset = world
        .resource::<Assets<PrecinctAsset>>()
        .get(instance.asset_id(server).ok()?)?;
    let (aspects, _) = instance.find(asset)?;
    let chain = Exemplar::chain(exemplars, instance.exemplar_id(world)?);
    Some(HeldAspects {
        kind: chain.first()?.1.kind,
        own: keys(&aspects.0),
        inherited: inherited(&chain),
        instance: true,
    })
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
    let Some(kind) = exemplars.get(this).map(|data| data.0.kind) else {
        return;
    };
    let paths: HashMap<String, String> = named_exemplars(&server, &exemplars, kind)
        .into_iter()
        .filter(|(_, candidate)| !Exemplar::chain_contains(&exemplars, *candidate, this))
        .filter_map(|(name, candidate)| Some((name, server.get_path(candidate)?.to_string())))
        .collect();
    let names = paths.keys().cloned().collect();
    commands
        .spawn_scene(chooser_dialog("Extends", "Choose", names))
        .insert(Name::new("ExtendsChooser"))
        .observe(
            move |chosen: On<ItemChosen>, mut edits: ResMut<PropertyEdits>| {
                if let Some(path) = paths.get(&chosen.name) {
                    let field = ParsedPath::empty().with_field("extends");
                    edits.set(&root, field, ExemplarRef(Some(path.clone())));
                }
            },
        );
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

/// The exemplars of a chain, each by its path, with its own aspects, nearest first.
type ChainAspects<'a> = Vec<(AssetPath<'static>, &'a [Arc<dyn Aspect>])>;

/// The exemplars `chain` holds, by path, with their own aspects.
fn chain_aspects<'a>(
    server: &AssetServer,
    chain: &[(AssetId<Exemplar>, &'a ExemplarData)],
) -> ChainAspects<'a> {
    chain
        .iter()
        .filter_map(|(id, data)| {
            let path = server.get_path(*id)?.clone_owned();
            Some((path, data.aspects.as_slice()))
        })
        .collect()
}

/// Whether `aspect` is scripted, declared in a module rather than a Rust type.
fn is_scripted(aspect: &dyn Aspect) -> bool {
    aspect
        .as_partial_reflect()
        .try_downcast_ref::<ScriptedAspect>()
        .is_some()
}

/// The roots of the panes of the aspects of a root keyed `holder`, which has `own` as its own and
/// inherits `chain`'s, nearest first: in the chain's order, from its farthest exemplar, each where
/// it first appears, then the own aspects nothing along the chain has. An own aspect's root is
/// `own_root`'s, given its key and whether it overrides an inherited one; an inherited aspect's
/// is from the nearest exemplar that has it. After each aspect come those it requires that the
/// resolved aspects lack. With `instance`, the root is an instance of `kind`: a required aspect
/// that cannot attach at its defaults says why, and a scripted aspect is neither overridden nor
/// added, since a precinct does not load scripted aspects.
// TODO(PAN135): scripted aspects on instances.
#[allow(clippy::too_many_arguments)]
pub(crate) fn aspect_roots(
    holder: &str,
    own: &[Arc<dyn Aspect>],
    chain: &ChainAspects,
    instance: bool,
    kind: Option<InstanceKind>,
    registry: &TypeRegistry,
    own_root: impl Fn(&str, bool) -> Arc<dyn PropertyRoot>,
) -> Vec<Arc<dyn PropertyRoot>> {
    let mut order: Vec<&str> = Vec::new();
    for (_, aspects) in chain.iter().rev() {
        for aspect in aspects.iter() {
            if !order.contains(&aspect.key()) {
                order.push(aspect.key());
            }
        }
    }
    for aspect in own {
        if !order.contains(&aspect.key()) {
            order.push(aspect.key());
        }
    }
    let own_index = |key: &str| own.iter().position(|aspect| aspect.key() == key);
    let mut missing: Vec<&str> = Vec::new();
    let mut roots: Vec<Arc<dyn PropertyRoot>> = Vec::new();
    for &key in &order {
        let from = chain.iter().find_map(|(path, aspects)| {
            let aspect = aspects.iter().find(|aspect| aspect.key() == key)?;
            Some((path, aspect))
        });
        let aspect = match (own_index(key), from) {
            (Some(index), _) => {
                roots.push(own_root(key, from.is_some()));
                &own[index]
            }
            (None, Some((path, aspect))) => {
                roots.push(Arc::new(InheritedAspect {
                    from: path.clone(),
                    aspect: key.to_string(),
                    overridable: !(instance && is_scripted(aspect.as_ref())),
                }));
                aspect
            }
            (None, None) => continue,
        };
        for required in aspect.requires() {
            if order.contains(&&**required) || missing.contains(&&**required) {
                continue;
            }
            missing.push(required);
            let scripted = aspect
                .required_decls()
                .iter()
                .any(|decl| decl.key == *required);
            let blocked = match instance {
                true => required_aspect(aspect.as_ref(), required, kind, registry).err(),
                false => None,
            };
            roots.push(Arc::new(MissingAspect {
                holder: holder.to_string(),
                aspect: required.to_string(),
                index: own_index(key).map_or(own.len(), |index| index + 1),
                blocked,
                addable: !(instance && scripted),
                decls: RequiredDecls(aspect.required_decls().to_vec()),
            }));
        }
    }
    roots
}

#[derive(SystemParam)]
struct InspectParams<'w, 's> {
    settings: Res<'w, EditorModeSettings>,
    choices: Res<'w, SceneryChoices>,
    selection: Res<'w, ScenerySelection>,
    precincts: Query<'w, 's, &'static Precinct>,
    assets: Res<'w, Assets<PrecinctAsset>>,
    server: Res<'w, AssetServer>,
    exemplars: Res<'w, Assets<Exemplar>>,
    registry: Res<'w, AppTypeRegistry>,
}

impl InspectParams<'_, '_> {
    /// The roots of the exemplar chosen in the current tool's list: the exemplar, then its
    /// aspects' panes.
    fn exemplar_roots(&self, tool: SceneryTool) -> Option<Vec<Arc<dyn PropertyRoot>>> {
        let name = self.choices.choice(tool)?;
        let id = chosen_exemplar(&self.server, &self.exemplars, tool, &name)?;
        let path = self.server.get_path(id)?.clone_owned();
        let chain = Exemplar::chain(&self.exemplars, id);
        let (_, data) = chain.first()?;
        let root = ExemplarRoot {
            exemplar: path.clone(),
            kind: kind(tool),
            name,
        };
        let holder = root.key();
        let mut roots: Vec<Arc<dyn PropertyRoot>> = vec![Arc::new(root)];
        let parents = chain_aspects(&self.server, &chain[1..]);
        roots.extend(aspect_roots(
            &holder,
            &data.aspects,
            &parents,
            false,
            data.kind,
            &self.registry.read(),
            |key, overrides| {
                Arc::new(ExemplarAspect {
                    exemplar: path.clone(),
                    aspect: key.to_string(),
                    overrides,
                })
            },
        ));
        Some(roots)
    }

    /// The roots of the instance selected: the instance, then its aspects' panes.
    fn instance_roots(&self) -> Option<Vec<Arc<dyn PropertyRoot>>> {
        self.selection.instance?;
        let precinct = self.precincts.get(self.selection.precinct?).ok()?;
        let handle = precinct.asset.as_ref()?;
        let asset = self.assets.get(handle)?;
        let iid = &self.selection.iid;
        let (list, index, exemplar, aspects) = match asset
            .scenery
            .iter()
            .position(|instance| instance.iid == *iid)
        {
            Some(index) => {
                let instance = &asset.scenery[index];
                let exemplar = asset.scenery_types.get(instance.id)?;
                (InstanceList::Scenery, index, exemplar, &instance.aspects)
            }
            None => {
                let index = asset.actors.iter().position(|actor| actor.iid == *iid)?;
                let actor = &asset.actors[index];
                (InstanceList::Actor, index, &actor.exemplar, &actor.aspects)
            }
        };
        let id = self
            .server
            .get_path_id(exemplar)?
            .try_typed::<Exemplar>()
            .ok()?;
        let chain = Exemplar::chain(&self.exemplars, id);
        let exemplar_kind = chain.first()?.1.kind;
        let kind = match (list, exemplar_kind) {
            (InstanceList::Actor, _) => InstanceKind::Actor,
            (_, Some(InstanceKind::Wall)) => InstanceKind::Wall,
            _ => InstanceKind::Fixture,
        };
        let root = InstanceRoot {
            precinct: self.server.get_path(handle)?.clone_owned(),
            list,
            index,
            exemplar: exemplar.clone(),
            kind: kind.name().to_string(),
            name: exemplar
                .rsplit_once('#')
                .map_or(exemplar.as_str(), |(_, label)| label)
                .to_string(),
        };
        let holder = root.key();
        let mut roots: Vec<Arc<dyn PropertyRoot>> = vec![Arc::new(root.clone())];
        let chain = chain_aspects(&self.server, &chain);
        roots.extend(aspect_roots(
            &holder,
            &aspects.0,
            &chain,
            true,
            Some(kind),
            &self.registry.read(),
            |key, overrides| {
                Arc::new(InstanceAspect {
                    instance: root.clone(),
                    aspect: key.to_string(),
                    overrides,
                })
            },
        ));
        Some(roots)
    }
}

/// Inspects the exemplar chosen in the current tool's list, or in selection mode the selected
/// instance, if any. A tool that places catalog entries is `catalog_entries`'. The roots are
/// replaced only when they change, keys, captions or panes.
fn inspect_scenery_roots(params: InspectParams, mut inspected: ResMut<InspectedRoots>) {
    let tool = params.settings.scenery_tool;
    if placed_entry(tool).is_some() {
        return;
    }
    let roots = match tool {
        SceneryTool::SelectInstance => params.instance_roots(),
        _ => params.exemplar_roots(tool),
    }
    .unwrap_or_default();
    let registry = params.registry.read();
    let same = inspected.roots.len() == roots.len()
        && inspected.roots.iter().zip(&roots).all(|(old, new)| {
            old.key() == new.key()
                && old.caption() == new.caption()
                && pane_role(&registry, old.as_ref()) == pane_role(&registry, new.as_ref())
        });
    if !same {
        inspected.roots = roots;
    }
}

#[derive(SystemParam)]
pub(crate) struct SceneryPropertiesParams<'w> {
    inspected: Res<'w, InspectedValues>,
    edits: ResMut<'w, PropertyEdits>,
    collapsed: ResMut<'w, CollapsedRows>,
    registry: Res<'w, AppTypeRegistry>,
}

impl ImmediateAttach<CapsPanel> for SceneryProperties {
    type Params = SceneryPropertiesParams<'static>;

    /// A pane for the first value, an exemplar, an instance or a catalog entry, filling the
    /// panel's last row: its header fixed, and its body scrolling, holding its own fields' rows,
    /// then, for a root that holds aspects, a subpane per aspect, the rest.
    fn construct(ui: &mut Imm<CapsPanel>, params: &mut SceneryPropertiesParams) {
        let edits = params.edits.as_mut();
        let collapsed = params.collapsed.as_mut();
        let registry = params.registry.read();
        let Some((first, aspects)) = params.inspected.values.split_first() else {
            return;
        };
        let (holder, fields) = (&first.root, &first.value);
        let kind = format!("{}:", holder.kind());
        let name = &holder.caption();
        let reflected: &dyn Reflect = holder.as_ref();
        let holds_aspects = registry
            .get_type_data::<ReflectAspectHolder>(Any::type_id(reflected))
            .is_some();
        ui.ch_id("pane")
            .on_spawn_apply_scene(|| bsn! { @pane() Node { flex_grow: 1.0, min_height: px(0) } })
            .add(|ui| {
                // "Fixture: WoodenChairSmall", the name in bold, then at the right the menu that
                // adds its absent fields and the button that opens the aspects chooser.
                ui.ch()
                    .on_spawn_apply_scene(pane_header)
                    .on_spawn_insert(|| Name::new("PaneHeader"))
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
                                    .on_spawn_apply_scene(|| {
                                        header_text(fonts::BOLD, FontWeight::BOLD)
                                    })
                                    .on_hash_change_text_fn(name, || name.clone());
                            });
                        ui.ch().on_spawn_insert(|| Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                        let fields = fields.as_ref();
                        root_add_menu(ui, holder, fields, &registry, edits, collapsed);
                        if holds_aspects
                            && icon_button(ui, "aspects", ASPECTS_ICON, px(18), tokens::TEXT_MAIN)
                        {
                            let root = holder.clone();
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
                                    holder,
                                    fields.as_ref(),
                                    aspects,
                                    &registry,
                                    edits,
                                    collapsed,
                                )
                            });
                    });
            });
    }
}

/// Part of the pane's caption, in `font` at the header's size: the header's own font does not
/// reach text inside a nested node.
pub(crate) fn header_text(font: &'static str, weight: FontWeight) -> impl Scene {
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

/// The holder's own fields' rows, then a subpane per aspect, in a column that scrolls, with its
/// scrollbar. A click in an aspect's header becomes an aspects edit of the holder: × reverts an
/// override or removes an aspect the holder added, the pencil overrides an inherited one, and +
/// adds a missing one.
fn scrolled_rows(
    ui: &mut Imm<CapsPanel>,
    holder: &Arc<dyn PropertyRoot>,
    fields: &dyn PartialReflect,
    aspects: &[InspectedValue],
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
    collapsed: &mut CollapsedRows,
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
            root_rows(ui, holder, fields, registry, edits, collapsed);
            for aspect in aspects {
                let pane = pane_role(registry, aspect.root.as_ref());
                let role = pane.as_ref().map(|(_, role)| role);
                let action = root_pane(ui, aspect, registry, edits, collapsed, role);
                let (Some(action), Some((key, role))) = (action, pane) else {
                    continue;
                };
                match (action, role) {
                    (PaneAction::Remove, PaneRole::Own { overrides }) => {
                        let kind = match overrides {
                            true => EditKind::Revert,
                            false => EditKind::Aspects,
                        };
                        edits.push_aspects(holder, kind, Vec::new(), vec![key]);
                    }
                    (PaneAction::Override, PaneRole::Inherited { .. }) => {
                        if let Some(copy) = as_aspect(registry, aspect.value.as_ref()) {
                            collapsed.expand_pane(&aspect.root.caption());
                            let add = vec![(usize::MAX, copy)];
                            edits.push_aspects(holder, EditKind::Override, add, Vec::new());
                        }
                    }
                    (PaneAction::Add, PaneRole::Missing { index, .. }) => {
                        if let Some(added) = as_aspect(registry, aspect.value.as_ref()) {
                            let add = vec![(index, added)];
                            edits.push_aspects(holder, EditKind::Aspects, add, Vec::new());
                        }
                    }
                    _ => {}
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

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    #[derive(Component, Reflect, Clone, Default)]
    struct Lock;

    #[derive(Component, Reflect, Clone, Default)]
    struct Container;

    #[derive(Component, Reflect, Clone, Default)]
    struct Light;

    #[derive(Component, Reflect, Clone, Default)]
    struct Tag;

    /// What `Lock` requires, which nothing here has, nor registers.
    static KEY: LazyLock<[Arc<str>; 1]> = LazyLock::new(|| [Arc::from("Key")]);

    macro_rules! test_aspect {
        ($type:ident, $requires:expr) => {
            impl panoply_exemplar::NativeAspect for $type {
                type Config = Self;

                fn config(&self) -> Self {
                    self.clone()
                }

                fn can_attach(&self, _kind: InstanceKind) -> bool {
                    true
                }

                fn requires(&self) -> &[Arc<str>] {
                    $requires
                }
            }
        };
    }

    test_aspect!(Lock, &*KEY);
    test_aspect!(Container, &[]);
    test_aspect!(Light, &[]);
    test_aspect!(Tag, &[]);

    /// The panes of a root that has `Container` and `Tag` as its own, and extends `Mid`, with
    /// `Light`, which extends `Base`, with `Lock` and `Container`: each pane's aspect and role.
    fn panes(instance: bool) -> Vec<(String, PaneRole)> {
        let mut registry = TypeRegistry::default();
        registry.register::<ExemplarAspect>();
        registry.register::<InheritedAspect>();
        registry.register::<MissingAspect>();
        let base: Vec<Arc<dyn Aspect>> = vec![Arc::new(Lock), Arc::new(Container)];
        let mid: Vec<Arc<dyn Aspect>> = vec![Arc::new(Light)];
        let own: Vec<Arc<dyn Aspect>> = vec![Arc::new(Tag), Arc::new(Container)];
        let chain: ChainAspects = vec![
            (AssetPath::from("e.exem.json#Mid"), mid.as_slice()),
            (AssetPath::from("e.exem.json#Base"), base.as_slice()),
        ];
        let roots = aspect_roots(
            "holder",
            &own,
            &chain,
            instance,
            Some(InstanceKind::Fixture),
            &registry,
            |key, overrides| {
                Arc::new(ExemplarAspect {
                    exemplar: AssetPath::from("e.exem.json#Root"),
                    aspect: key.to_string(),
                    overrides,
                })
            },
        );
        roots
            .iter()
            .map(|root| pane_role(&registry, root.as_ref()).unwrap())
            .collect()
    }

    /// From the farthest exemplar on, each aspect where it first appears, an override in its
    /// inherited place and an added one last, and a missing one after its requirer, blocked only
    /// on an instance, since `Key` cannot attach at its defaults.
    #[test]
    fn aspects_are_in_chain_order() {
        let inherited = PaneRole::Inherited { overridable: true };
        let missing = |blocked: Option<&str>| PaneRole::Missing {
            index: 2,
            blocked: blocked.map(str::to_string),
            addable: true,
        };
        let expected = |blocked| {
            vec![
                ("Lock".to_string(), inherited.clone()),
                ("Key".to_string(), missing(blocked)),
                ("Container".to_string(), PaneRole::Own { overrides: true }),
                ("Light".to_string(), inherited.clone()),
                ("Tag".to_string(), PaneRole::Own { overrides: false }),
            ]
        };
        assert_eq!(panes(false), expected(None));
        assert_eq!(panes(true), expected(Some("it is not registered")));
    }
}
