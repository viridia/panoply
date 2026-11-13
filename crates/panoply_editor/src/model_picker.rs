//! The dialog a model field's button opens: the models of the field's kind (an actor's body, a
//! wall's or fixture's model), a disclosure row per glTF file holding a row per named scene in it,
//! with the field's model selected and its file open; for a clips field, a row per character file
//! holding clips. A click
//! on a file's row opens or closes it; Save, or a double-click on a model, sets the field. Cancel,
//! Esc or a click outside closes it.

use bevy::{
    asset::LoadedFolder,
    feathers::{
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody,
            FeathersDialogClose, FeathersDialogFooter, FeathersDialogHeader, FeathersScrollbar,
            FeathersTreeItem,
        },
        display::caption,
        theme::ThemedText,
    },
    gltf::Gltf,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    ui::{Expanded, InteractionDisabled, ScrollIntoView, ScrollIntoViewContainer},
    ui_widgets::{
        tree_view_expand_self_update, tree_view_self_update, Activate, ControlOrientation,
        RequestClose, ScrollArea, SelectedTreeItem, TreeItem, TreeItemActivate,
        TreeItemExpandChange, TreeView,
    },
};

use panoply_actors::{ClipsFile, Skin};
use panoply_scenery::SceneryModel;

use crate::toast::ShowToast;

/// The scene Blender's glTF export adds to every file, holding all of its objects: not a model.
const DEFAULT_SCENE: &str = "Scene";

/// What a model field names, and so the folders its picker lists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModelKind {
    /// An actor's body (`Skin`).
    Character,
    /// A wall's or a fixture's model (`SceneryModel`).
    Scenery,
    /// A character file holding clips (`ClipsFile`): the file itself, not a scene in it.
    Clips,
}

impl ModelKind {
    const ALL: [ModelKind; 3] = [ModelKind::Character, ModelKind::Scenery, ModelKind::Clips];

    /// The folders holding this kind's glTF files.
    fn folders(self) -> &'static [&'static str] {
        match self {
            ModelKind::Character | ModelKind::Clips => &["characters"],
            ModelKind::Scenery => &["scenery/models", "terrain/models"],
        }
    }
}

/// The kind of model `value` names, and its path, if it is a model field.
pub(crate) fn model_field(value: &dyn PartialReflect) -> Option<(ModelKind, &str)> {
    if let Some(skin) = value.try_downcast_ref::<Skin>() {
        Some((ModelKind::Character, &skin.0))
    } else if let Some(clips) = value.try_downcast_ref::<ClipsFile>() {
        Some((ModelKind::Clips, &clips.0))
    } else {
        let model = value.try_downcast_ref::<SceneryModel>()?;
        Some((ModelKind::Scenery, &model.0))
    }
}

pub struct ModelPickerPlugin;

impl Plugin for ModelPickerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_model_folders).add_systems(
            Update,
            (select_current_model, scroll_to_model, enable_save),
        );
    }
}

/// Keeps every kind's model folders loaded, so a picker lists them all as it opens.
#[derive(Resource)]
pub(crate) struct PickerFolders(Vec<(&'static str, Handle<LoadedFolder>)>);

fn load_model_folders(mut commands: Commands, server: Res<AssetServer>) {
    let mut paths: Vec<&'static str> = ModelKind::ALL
        .iter()
        .flat_map(|kind| kind.folders())
        .copied()
        .collect();
    paths.sort();
    paths.dedup();
    let folders = paths
        .into_iter()
        .map(|path| (path, server.load_folder(path)))
        .collect();
    commands.insert_resource(PickerFolders(folders));
}

/// On a model field's button: the model last chosen for it, until the panel takes it.
#[derive(Component, Default, Clone)]
pub struct ChosenModel(pub Option<String>);

/// A picker's root: the button it sets.
#[derive(Component)]
struct ModelPicker {
    button: Entity,
}

/// On a picker until its field's model row is found and selected: that model.
#[derive(Component)]
struct SelectModel(String);

/// The tree of a picker's files and models.
#[derive(Component, Default, Clone)]
struct ModelTree;

/// A model's row, holding the field's value for it: `file#Scene`.
#[derive(Component, Default, Clone)]
struct ModelRow(String);

/// A row selected as the picker opens, to scroll into view once it has been laid out.
#[derive(Component)]
struct ScrollToModel;

/// A picker's Save button, disabled while no model is selected.
#[derive(Component, Default, Clone)]
struct SaveModel;

/// A glTF file, its path, and the field's value for each of its named scenes, sorted: none for a
/// clips field, which names the file.
struct ModelFile {
    name: String,
    path: String,
    models: Vec<String>,
}

/// Opens a picker of `kind`'s models for the button `button`, whose field holds `current`. If
/// the models have not all loaded yet, says so instead.
pub(crate) fn open_model_picker(
    In((button, kind, current)): In<(Entity, ModelKind, String)>,
    picker_folders: Res<PickerFolders>,
    folders: Res<Assets<LoadedFolder>>,
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut toasts: MessageWriter<ShowToast>,
    mut commands: Commands,
) {
    let loaded: Option<Vec<&LoadedFolder>> = picker_folders
        .0
        .iter()
        .filter(|(path, _)| kind.folders().contains(path))
        .map(|(_, handle)| {
            folders
                .get(handle)
                .filter(|_| server.is_loaded_with_dependencies(handle))
        })
        .collect();
    let Some(loaded) = loaded else {
        toasts.write(ShowToast("The models are still loading".into()));
        return;
    };
    let mut files: Vec<ModelFile> = loaded
        .iter()
        .flat_map(|folder| &folder.handles)
        .filter_map(|handle| {
            let id = handle.id().try_typed::<Gltf>().ok()?;
            let path = server.get_path(id)?;
            let name = path.path().file_stem()?.to_string_lossy().into_owned();
            let gltf = gltfs.get(id)?;
            if kind == ModelKind::Clips {
                return (!gltf.named_animations.is_empty()).then(|| ModelFile {
                    name,
                    path: path.to_string(),
                    models: Vec::new(),
                });
            }
            let mut models: Vec<String> = gltf
                .named_scenes
                .keys()
                .filter(|scene| scene.as_ref() != DEFAULT_SCENE)
                .map(|scene| format!("{path}#{scene}"))
                .collect();
            models.sort_by_key(|model| model.to_lowercase());
            Some(ModelFile {
                name,
                path: path.to_string(),
                models,
            })
        })
        .collect();
    files.sort_by_key(|file| file.name.to_lowercase());
    let rows: Box<dyn SceneList> = if kind == ModelKind::Clips {
        Box::new(
            files
                .iter()
                .map(|file| model_row(&file.name, &file.path))
                .collect::<Vec<_>>(),
        )
    } else {
        Box::new(files.iter().map(file_row).collect::<Vec<_>>())
    };
    let title = if kind == ModelKind::Clips {
        "Choose clips"
    } else {
        "Choose model"
    };
    commands
        .spawn_scene(picker_dialog(title, rows))
        .insert((
            Name::new("ModelPicker"),
            ModelPicker { button },
            SelectModel(current),
        ));
}

/// The picker, titled `title`, listing `rows`.
fn picker_dialog(title: &'static str, rows: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: { Box::new(dialog_contents(title, rows)) as Box<dyn SceneList> },
        }
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
    }
}

/// The contents, in a function of their own, as `chooser_dialog`'s are: a `bsn_list!` written
/// inside the dialog's `bsn!` can give a child the root's entity.
fn dialog_contents(title: &'static str, rows: Box<dyn SceneList>) -> impl SceneList {
    bsn_list! {
        @FeathersDialogHeader Children [
            Text(title) ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Node { height: vh(60), min_height: px(0) }
        Children [ @model_tree(rows) ]
        --
        @FeathersDialogFooter Children [
            @FeathersButton { @caption: bsn! { Text("Cancel") ThemedText } }
            #Cancel
            on(|activate: On<Activate>, mut commands: Commands| {
                commands.trigger(RequestClose { source: activate.event_target() });
            })
            --
            @FeathersButton {
                @caption: bsn! { Text("Save") ThemedText },
                @variant: ButtonVariant::Primary,
            }
            #Save
            SaveModel
            InteractionDisabled
            on(save_selected)
        ]
    }
}

/// The tree, scrolling, with its scrollbar at its right. The scrolling node is the tree itself,
/// so that the rows are its children, where the tree looks for them to indent and to step through
/// with the arrow keys: feathers' tree view puts them a node further down.
fn model_tree(rows: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node { flex_grow: 1.0, min_height: px(0) }
        Children [
            #Tree
            ModelTree
            TreeView
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::right(px(14)),
                overflow: Overflow::scroll_y(),
            }
            ScrollArea
            on(tree_view_self_update)
            on(tree_view_expand_self_update)
            on(toggle_file)
            on(save_double_clicked)
            Children [ {rows} ]
            --
            @FeathersScrollbar {
                @target: #Tree,
                @orientation: {ControlOrientation::Vertical}
            }
            Node {
                position_type: PositionType::Absolute,
                right: px(4),
                top: px(0),
                bottom: px(0),
                width: px(6),
            }
        ]
    }
}

/// A file's row, named after it, holding a row per model.
fn file_row(file: &ModelFile) -> impl Scene {
    let name = file.name.clone();
    let models: Vec<_> = file
        .models
        .iter()
        .map(|model| {
            let scene = model.rsplit_once('#').map_or(model.as_str(), |(_, scene)| scene);
            model_row(scene, model)
        })
        .collect();
    let models: Box<dyn SceneList> = Box::new(models);
    bsn! {
        @FeathersTreeItem {
            @expandable: true,
            @label: bsn_list! { @caption(name.clone()) },
            @children: {models},
        }
        Name({name.clone()})
    }
}

/// A row choosing `model`, captioned and named `name`: a model's scene, under its file's row, so
/// `canine/Canine` to a plan, or a clips file's name.
fn model_row(name: &str, model: &str) -> impl Scene {
    let name = name.to_string();
    let model = model.to_string();
    bsn! {
        @FeathersTreeItem { @label: bsn_list! { @caption(name.clone()) } }
        ModelRow({model.clone()})
        Name({name.clone()})
    }
}

/// Observer on the tree: a click on a file's row opens or closes it, as its disclosure toggle
/// does.
fn toggle_file(
    activate: On<TreeItemActivate>,
    models: Query<(), With<ModelRow>>,
    open: Query<Has<Expanded>>,
    mut commands: Commands,
) {
    if models.contains(activate.item) {
        return;
    }
    let Ok(expanded) = open.get(activate.item) else {
        return;
    };
    commands.trigger(TreeItemExpandChange {
        tree: activate.tree,
        item: activate.item,
        expanded: !expanded,
    });
}

/// Observer on the tree: a double-click on a model's row chooses it.
fn save_double_clicked(
    click: On<PointerClick>,
    models: Query<&ModelRow>,
    parents: Query<&ChildOf>,
    pickers: Query<&ModelPicker>,
    mut buttons: Query<&mut ChosenModel>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary || click.count < 2 {
        return;
    }
    let target = click.original_event_target();
    let Some(model) = std::iter::once(target)
        .chain(parents.iter_ancestors(target))
        .find_map(|e| models.get(e).ok())
    else {
        return;
    };
    let tree = click.event_target();
    choose(tree, &model.0, &parents, &pickers, &mut buttons, &mut commands);
}

/// Observer on Save: chooses the selected model.
fn save_selected(
    activate: On<Activate>,
    trees: Query<(&SelectedTreeItem, &ChildOf), With<ModelTree>>,
    children: Query<&Children>,
    models: Query<&ModelRow>,
    parents: Query<&ChildOf>,
    pickers: Query<&ModelPicker>,
    mut buttons: Query<&mut ChosenModel>,
    mut commands: Commands,
) {
    let Some(picker) = parents
        .iter_ancestors(activate.event_target())
        .find(|&e| pickers.contains(e))
    else {
        return;
    };
    let Some((tree, model)) = children.iter_descendants(picker).find_map(|e| {
        let (selected, _) = trees.get(e).ok()?;
        Some((e, models.get(selected.0?).ok()?))
    }) else {
        return;
    };
    choose(tree, &model.0, &parents, &pickers, &mut buttons, &mut commands);
}

/// Gives `model` to the button of the picker holding `tree`, and closes the picker.
fn choose(
    tree: Entity,
    model: &str,
    parents: &Query<&ChildOf>,
    pickers: &Query<&ModelPicker>,
    buttons: &mut Query<&mut ChosenModel>,
    commands: &mut Commands,
) {
    let Some((dialog, picker)) = parents
        .iter_ancestors(tree)
        .find_map(|e| pickers.get(e).ok().map(|picker| (e, picker)))
    else {
        return;
    };
    // The button is gone if the panel moved on while the picker was open.
    if let Ok(mut chosen) = buttons.get_mut(picker.button) {
        chosen.0 = Some(model.to_string());
    }
    commands.entity(dialog).despawn();
}

/// Selects the field's model in each picker that has just opened, once its row has spawned, and
/// opens its file.
fn select_current_model(
    pickers: Query<(Entity, &SelectModel)>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    trees: Query<(), With<ModelTree>>,
    models: Query<&ModelRow>,
    files: Query<(), (With<TreeItem>, Without<ModelRow>)>,
    mut commands: Commands,
) {
    for (picker, wanted) in &pickers {
        let Some(tree) = children.iter_descendants(picker).find(|&e| trees.contains(e)) else {
            continue;
        };
        let Some(row) = children
            .iter_descendants(tree)
            .find(|&e| models.get(e).is_ok_and(|model| model.0 == wanted.0))
        else {
            // A field naming no model there is nothing to select.
            if children.iter_descendants(tree).next().is_some() {
                commands.entity(picker).remove::<SelectModel>();
            }
            continue;
        };
        if let Some(file) = parents.iter_ancestors(row).find(|&e| files.contains(e)) {
            commands.entity(file).insert(Expanded);
        }
        commands.entity(tree).insert(SelectedTreeItem(Some(row)));
        commands.entity(row).insert(ScrollToModel);
        commands.entity(picker).remove::<SelectModel>();
    }
}

/// Scrolls each row selected as its picker opened into view, once layout has given it a size.
fn scroll_to_model(
    rows: Query<(Entity, &ComputedNode), With<ScrollToModel>>,
    mut commands: Commands,
) {
    for (row, node) in &rows {
        if node.size().y > 0.0 {
            commands.entity(row).remove::<ScrollToModel>();
            commands.trigger(ScrollIntoView {
                entity: row,
                container: ScrollIntoViewContainer::Nearest,
            });
        }
    }
}

/// Disables each picker's Save while no model is selected: a file's row may be.
fn enable_save(
    pickers: Query<Entity, With<ModelPicker>>,
    children: Query<&Children>,
    trees: Query<&SelectedTreeItem, With<ModelTree>>,
    models: Query<(), With<ModelRow>>,
    buttons: Query<Has<InteractionDisabled>, With<SaveModel>>,
    mut commands: Commands,
) {
    for picker in &pickers {
        let Some(selected) = children
            .iter_descendants(picker)
            .find_map(|e| trees.get(e).ok())
        else {
            continue;
        };
        let model = selected.0.is_some_and(|row| models.contains(row));
        for button in children.iter_descendants(picker) {
            match (model, buttons.get(button)) {
                (true, Ok(true)) => {
                    commands.entity(button).remove::<InteractionDisabled>();
                }
                (false, Ok(false)) => {
                    commands.entity(button).insert(InteractionDisabled);
                }
                _ => {}
            }
        }
    }
}
