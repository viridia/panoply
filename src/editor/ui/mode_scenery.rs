use crate::{
    actors::ACTOR_TYPE,
    editor::{
        events::{PlaceWalls, RemoveWalls, RotateSelection},
        modified_assets::ModifiedAssets,
        undo::{UndoEntry, UndoStack},
        EditorMode,
    },
    scenery::{
        precinct::Precinct,
        precinct_asset::{PrecinctAsset, SceneryInstanceData, SceneryInstanceId},
        scenery_element::{SceneryElement, SceneryElementRebuildAspects},
        FIXTURE_TYPE, FLOOR_TYPE, PRECINCT_SIZE_F, TIER_OFFSET, WALL_TYPE,
    },
};
use bevy::{prelude::*, render::view::RenderLayers, ui, utils::hashbrown::HashSet};
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey};
use bevy_quill::prelude::*;
use bevy_quill_obsidian::{prelude::*, RoundedCorners};
use panoply_exemplar::Exemplar;

use super::{controls::ExemplarChooser, tool_floor_create, tool_floor_edit, tool_wall_create};

pub(crate) struct EditSceneryPlugin;

impl Plugin for EditSceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(SceneryTool::default())
            .insert_state(FloorTool::default())
            .insert_state(WallSnap::default())
            .add_computed_state::<SceneryOverlay>()
            .enable_state_scoped_entities::<SceneryTool>()
            .enable_state_scoped_entities::<FloorTool>()
            .enable_state_scoped_entities::<WallSnap>()
            .init_resource::<SelectedPrecinct>()
            .init_resource::<SelectedTier>()
            .init_resource::<SelectedFacing>()
            .init_resource::<SceneryDragState>()
            .init_resource::<FloorType>()
            .init_resource::<FloorFilter>()
            .init_resource::<WallType>()
            .init_resource::<WallFilter>()
            .init_resource::<FixtureType>()
            .init_resource::<FixtureFilter>()
            .init_resource::<ActorType>()
            .init_resource::<ActorFilter>()
            .register_type::<State<SceneryTool>>()
            .register_type::<NextState<SceneryTool>>()
            .register_type::<State<FloorTool>>()
            .register_type::<NextState<FloorTool>>()
            .register_type::<State<WallSnap>>()
            .register_type::<NextState<WallSnap>>()
            .register_type::<FloorType>()
            .register_type::<FloorFilter>()
            .register_type::<WallType>()
            .register_type::<WallFilter>()
            .register_type::<FixtureType>()
            .register_type::<FixtureFilter>()
            .register_type::<ActorType>()
            .register_type::<ActorFilter>()
            .register_type::<SelectedTier>()
            .register_type::<SelectedFacing>()
            .add_systems(
                OnEnter(SceneryOverlay::FloorCreate),
                tool_floor_create::enter,
            )
            .add_systems(OnExit(SceneryOverlay::FloorCreate), tool_floor_create::exit)
            .add_systems(OnEnter(SceneryOverlay::FloorDraw), tool_floor_edit::enter)
            .add_systems(OnExit(SceneryOverlay::FloorDraw), tool_floor_edit::exit)
            .add_systems(OnEnter(SceneryOverlay::PlaceWall), tool_wall_create::enter)
            .add_systems(OnExit(SceneryOverlay::PlaceWall), tool_wall_create::exit)
            .add_systems(
                Update,
                (
                    tool_floor_create::update.run_if(in_state(SceneryOverlay::FloorCreate)),
                    tool_floor_edit::update.run_if(in_state(SceneryOverlay::FloorDraw)),
                    tool_wall_create::update.run_if(in_state(SceneryOverlay::PlaceWall)),
                    update.run_if(in_state(EditorMode::Scenery)),
                ),
            )
            .observe(place_walls)
            .observe(remove_walls);
    }
}

#[derive(Resource, Default)]
pub struct SelectedPrecinct(pub Option<Entity>);

#[derive(Resource, Default, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("selected_tier"))]
pub struct SelectedTier(pub i16);

#[derive(Resource, Default, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("selected_facing"))]
pub struct SelectedFacing(pub i32);

#[derive(Resource, Default, Reflect)]
// #[reflect(@PreferencesGroup("editor"), @PreferencesKey("floor_type"))]
pub struct FloorType(pub Option<AssetId<Exemplar>>);

#[derive(Resource, Default, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("floor_type_filter"))]
pub struct FloorFilter(pub String);

#[derive(Resource, Default, Reflect)]
// #[reflect(@PreferencesGroup("editor"), @PreferencesKey("wall_type"))]
pub struct WallType(pub Option<AssetId<Exemplar>>);

#[derive(Resource, Default, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("wall_type_filter"))]
pub struct WallFilter(pub String);

#[derive(Resource, Default, Reflect)]
// #[reflect(@PreferencesGroup("editor"), @PreferencesKey("fixture_type"))]
pub struct FixtureType(pub Option<AssetId<Exemplar>>);

#[derive(Resource, Default, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("fixture_type_filter"))]
pub struct FixtureFilter(pub String);

#[derive(Resource, Default, Reflect)]
// #[reflect(@PreferencesGroup("editor"), @PreferencesKey("actor_type"))]
pub struct ActorType(pub Option<AssetId<Exemplar>>);

#[derive(Resource, Default, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("wall_type_filter"))]
pub struct ActorFilter(pub String);

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("scenery_tool"))]
pub enum SceneryTool {
    #[default]
    FloorDraw,
    WallDraw,
    FixtureDraw,
    ActorPlacement,
    TerrainFxDraw,
    SceneryEdit,
    EditLayers,
    SceneryRect,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("floor_tool"))]
pub enum FloorTool {
    #[default]
    Move,
    Draw,
    RectM,
    RectL,
    RectXL,
    Beveled,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("wall_snap"))]
pub enum WallSnap {
    #[default]
    Normal,
    Offset,
    Quarter,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SceneryOverlay {
    FloorDraw,
    FloorCreate,
    PlaceWall,
    PlaceFixture,
    PlaceActor,
    DrawTerrainFx,
    Interact,
    RectSelect,
}

#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct SceneryDragState {
    pub(crate) dragging: bool,
    pub(crate) precinct: Option<Entity>,
    pub(crate) anchor_pos: Vec2,
    pub(crate) cursor_pos: Vec2,
    pub(crate) anchor_height: i32,
    pub(crate) floor_outline: Vec<Vec2>,
    pub(crate) cursor_exemplar: Option<AssetId<Exemplar>>,
    pub(crate) cursor_model: Option<Entity>,
    pub(crate) cursor_layer: usize,
}

impl ComputedStates for SceneryOverlay {
    type SourceStates = (EditorMode, SceneryTool, FloorTool);

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        if sources.0 != EditorMode::Scenery {
            return None;
        }
        match sources.1 {
            SceneryTool::FloorDraw => match sources.2 {
                FloorTool::Move | FloorTool::Draw => Some(SceneryOverlay::FloorDraw),
                FloorTool::RectM | FloorTool::RectL | FloorTool::RectXL | FloorTool::Beveled => {
                    Some(SceneryOverlay::FloorCreate)
                }
            },
            SceneryTool::WallDraw => Some(SceneryOverlay::PlaceWall),
            SceneryTool::FixtureDraw => Some(SceneryOverlay::PlaceFixture),
            SceneryTool::ActorPlacement => Some(SceneryOverlay::PlaceActor),
            SceneryTool::TerrainFxDraw => Some(SceneryOverlay::DrawTerrainFx),
            SceneryTool::SceneryEdit => Some(SceneryOverlay::Interact),
            SceneryTool::SceneryRect => Some(SceneryOverlay::RectSelect),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct EditModeSceneryControls;

impl ViewTemplate for EditModeSceneryControls {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<SceneryTool>>().get();

        Element::<NodeBundle>::new().style(style_panel).children((
            ToolPalette::new()
                .columns(2)
                .size(Size::Xl)
                .style(|sb: &mut StyleBuilder| {
                    sb.grid_column_start(1).grid_row_start(1);
                })
                .children((
                    ToolIconButton::new("editor/icons/floor-draw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::TopLeft)
                        .selected(st == SceneryTool::FloorDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::FloorDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/wall-draw.png")
                        .size(Vec2::new(24., 24.))
                        .tint(false)
                        .corners(RoundedCorners::TopRight)
                        .selected(st == SceneryTool::WallDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::WallDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/furnishing-draw.png")
                        .size(Vec2::new(20., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::FixtureDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::FixtureDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/actor.png")
                        .size(Vec2::new(24., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::ActorPlacement)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::ActorPlacement);
                            },
                        )),
                    ToolIconButton::new("editor/icons/road-draw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::TerrainFxDraw)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::TerrainFxDraw);
                            },
                        )),
                    ToolIconButton::new("editor/icons/machine.png")
                        .size(Vec2::new(28., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::SceneryEdit)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::SceneryEdit);
                            },
                        )),
                    ToolIconButton::new("editor/icons/layers.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::EditLayers)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::EditLayers);
                            },
                        )),
                    ToolIconButton::new("editor/icons/rect-select.png")
                        .size(Vec2::new(28., 24.))
                        .tint(false)
                        .selected(st == SceneryTool::SceneryRect)
                        .on_click(cx.create_callback(
                            |mut mode: ResMut<NextState<SceneryTool>>| {
                                mode.set(SceneryTool::SceneryRect);
                            },
                        )),
                    ToolIconButton::new("editor/icons/rotate-ccw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::BottomLeft)
                        .on_click(cx.create_callback(|mut commands: Commands| {
                            commands.trigger(RotateSelection(-1));
                        })),
                    ToolIconButton::new("editor/icons/rotate-cw.png")
                        .size(Vec2::new(32., 24.))
                        .tint(false)
                        .corners(RoundedCorners::BottomRight)
                        .on_click(cx.create_callback(|mut commands: Commands| {
                            commands.trigger(RotateSelection(1));
                        })),
                )),
            ToolPalette::new()
                .columns(3)
                .style(|sb: &mut StyleBuilder| {
                    sb.grid_column_start(1).grid_row_start(2);
                })
                .children((
                    ToolButton::new()
                        .children("Cut")
                        .corners(RoundedCorners::Left)
                        .selected(st == SceneryTool::FloorDraw),
                    ToolIconButton::new(
                        "embedded://bevy_quill_obsidian/assets/icons/chevron_down.png",
                    )
                    .size(Vec2::new(16., 16.))
                    .on_click(cx.create_callback(
                        |mut selected_tier: ResMut<SelectedTier>| {
                            selected_tier.0 = (selected_tier.0 - 1).clamp(-8, 16);
                        },
                    )),
                    ToolIconButton::new(
                        "embedded://bevy_quill_obsidian/assets/icons/chevron_up.png",
                    )
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Right)
                    .on_click(cx.create_callback(
                        |mut selected_tier: ResMut<SelectedTier>| {
                            selected_tier.0 = (selected_tier.0 + 1).clamp(-8, 16);
                        },
                    )),
                )),
            ListView::new().style(|sb: &mut StyleBuilder| {
                sb.grid_row_start(3).grid_row_end(4).min_height(48);
            }),
            Element::<NodeBundle>::new()
                .style(style_chooser_panel)
                .children((Switch::new(st)
                    .case(
                        SceneryTool::FloorDraw,
                        (FloorToolSelector, FloorExemplarChooser),
                    )
                    .case(
                        SceneryTool::WallDraw,
                        (WallSnapSelector, WallExemplarChooser),
                    )
                    .case(SceneryTool::FixtureDraw, FixtureExemplarChooser)
                    .case(SceneryTool::ActorPlacement, ActorExemplarChooser)
                    .fallback(()),)),
        ))
    }
}

fn style_panel(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Grid)
        .grid_template_columns(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .grid_template_rows(vec![
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::auto(1),
            ui::RepeatedGridTrack::flex(1, 1.),
        ])
        .gap(8)
        .flex_grow(1.);
}

fn style_chooser_panel(ss: &mut StyleBuilder) {
    ss.display(ui::Display::Flex)
        .flex_direction(ui::FlexDirection::Column)
        .align_items(ui::AlignItems::Stretch)
        .gap(8)
        .min_height(0)
        .grid_row_start(1)
        .grid_row_span(3)
        .grid_column_start(2)
        .grid_column_span(1);
}

fn style_exemplar_chooser(ss: &mut StyleBuilder) {
    ss.min_height(0).flex_grow(1.);
}

#[derive(Clone, PartialEq)]
pub(crate) struct FloorToolSelector;

impl ViewTemplate for FloorToolSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<FloorTool>>().get();

        ToolPalette::new()
            .columns(6)
            .size(Size::Lg)
            .style(|sb: &mut StyleBuilder| {
                sb.align_self(ui::AlignSelf::Start);
            })
            .children((
                ToolIconButton::new("editor/icons/pointer.png")
                    .size(Vec2::new(13., 16.))
                    .corners(RoundedCorners::Left)
                    .selected(st == FloorTool::Move)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::Move);
                        }),
                    ),
                ToolIconButton::new("editor/icons/pencil.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::Draw)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::Draw);
                        }),
                    ),
                ToolIconButton::new("editor/icons/tile1.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::RectM)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::RectM);
                        }),
                    ),
                ToolIconButton::new("editor/icons/tile2.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::RectL)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::RectL);
                        }),
                    ),
                ToolIconButton::new("editor/icons/tile3.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == FloorTool::RectXL)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::RectXL);
                        }),
                    ),
                ToolIconButton::new("editor/icons/octagon.png")
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Right)
                    .selected(st == FloorTool::Beveled)
                    .on_click(
                        cx.create_callback(|mut mode: ResMut<NextState<FloorTool>>| {
                            mode.set(FloorTool::Beveled);
                        }),
                    ),
            ))
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct WallSnapSelector;

impl ViewTemplate for WallSnapSelector {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let st = *cx.use_resource::<State<WallSnap>>().get();

        ToolPalette::new()
            .columns(3)
            // .size(Size::Xl)
            .style(|sb: &mut StyleBuilder| {
                sb.align_self(ui::AlignSelf::Start);
            })
            .children((
                ToolIconButton::new("editor/icons/grid-normal.png")
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Left)
                    .selected(st == WallSnap::Normal)
                    .on_click(cx.create_callback(|mut mode: ResMut<NextState<WallSnap>>| {
                        mode.set(WallSnap::Normal);
                    })),
                ToolIconButton::new("editor/icons/grid-offset.png")
                    .size(Vec2::new(16., 16.))
                    .selected(st == WallSnap::Offset)
                    .on_click(cx.create_callback(|mut mode: ResMut<NextState<WallSnap>>| {
                        mode.set(WallSnap::Offset);
                    })),
                ToolIconButton::new("editor/icons/grid-fine.png")
                    .size(Vec2::new(16., 16.))
                    .corners(RoundedCorners::Right)
                    .selected(st == WallSnap::Quarter)
                    .on_click(cx.create_callback(|mut mode: ResMut<NextState<WallSnap>>| {
                        mode.set(WallSnap::Quarter);
                    })),
            ))
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct FloorExemplarChooser;

impl ViewTemplate for FloorExemplarChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = cx.create_callback(
            |key: In<Option<AssetId<Exemplar>>>, mut selected: ResMut<FloorType>| {
                selected.0 = *key;
            },
        );
        let selected = cx.use_resource::<FloorType>();
        let filter = cx.use_resource::<FloorFilter>();
        ExemplarChooser {
            selected: selected.0,
            instance_type: FLOOR_TYPE,
            filter: filter.0.clone(),
            style: style_exemplar_chooser.into_handle(),
            on_change,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct WallExemplarChooser;

impl ViewTemplate for WallExemplarChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = cx.create_callback(
            |key: In<Option<AssetId<Exemplar>>>, mut selected: ResMut<WallType>| {
                selected.0 = *key;
            },
        );
        let selected = cx.use_resource::<WallType>();
        let filter = cx.use_resource::<WallFilter>();
        ExemplarChooser {
            selected: selected.0,
            instance_type: WALL_TYPE,
            filter: filter.0.clone(),
            style: style_exemplar_chooser.into_handle(),
            on_change,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct FixtureExemplarChooser;

impl ViewTemplate for FixtureExemplarChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = cx.create_callback(
            |key: In<Option<AssetId<Exemplar>>>, mut selected: ResMut<FixtureType>| {
                selected.0 = *key;
            },
        );
        let selected = cx.use_resource::<FixtureType>();
        let filter = cx.use_resource::<FixtureFilter>();
        ExemplarChooser {
            selected: selected.0,
            instance_type: FIXTURE_TYPE,
            filter: filter.0.clone(),
            style: style_exemplar_chooser.into_handle(),
            on_change,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct ActorExemplarChooser;

impl ViewTemplate for ActorExemplarChooser {
    type View = impl View;

    fn create(&self, cx: &mut Cx) -> Self::View {
        let on_change = cx.create_callback(
            |key: In<Option<AssetId<Exemplar>>>, mut selected: ResMut<ActorType>| {
                selected.0 = *key;
            },
        );
        let selected = cx.use_resource::<ActorType>();
        let filter = cx.use_resource::<ActorFilter>();
        ExemplarChooser {
            selected: selected.0,
            instance_type: ACTOR_TYPE,
            filter: filter.0.clone(),
            style: style_exemplar_chooser.into_handle(),
            on_change,
        }
    }
}

fn update(
    mut commands: Commands,
    q_precints: Query<&Precinct>,
    q_children: Query<&Children>,
    mut q_scenery_elements: Query<&mut SceneryElement>,
    mut r_drag_state: ResMut<SceneryDragState>,
    mut r_exemplars: ResMut<Assets<Exemplar>>,
    r_selected_tier: Res<SelectedTier>,
    r_selected_facing: Res<SelectedFacing>,
) {
    let mut show_wall = false;
    if let Some(exemplar_id) = r_drag_state.cursor_exemplar {
        if let Some(ref exemplar) = r_exemplars.get_strong_handle(exemplar_id) {
            let precinct = q_precints.get(r_drag_state.precinct.unwrap()).unwrap();
            show_wall = true;
            let facing = r_selected_facing.0 as f32 * -std::f32::consts::PI / 2.;

            let mut coords: Vec<Vec3> = Vec::with_capacity(64);
            let area = Rect::from_corners(r_drag_state.anchor_pos, r_drag_state.cursor_pos);
            let mut x = area.min.x;
            while x <= area.max.x {
                let mut z = area.min.y;
                while z <= area.max.y {
                    coords.push(Vec3::new(x, 0., z));
                    z += 1.0;
                }
                x += 1.0;
            }

            let cursor_model = match r_drag_state.cursor_model {
                Some(parent) => parent,
                None => commands.spawn(SpatialBundle::default()).id(),
            };
            r_drag_state.cursor_model = Some(cursor_model);
            commands
                .entity(cursor_model)
                .insert(Transform::from_translation(Vec3::new(
                    precinct.coords.x as f32 * PRECINCT_SIZE_F,
                    r_selected_tier.0 as f32 + TIER_OFFSET,
                    precinct.coords.y as f32 * PRECINCT_SIZE_F,
                )));

            let mut children = match q_children.get(cursor_model) {
                Ok(children) => children.iter().copied().collect(),
                Err(_) => Vec::new(),
            };

            if children.len() > coords.len() {
                for child in children.iter().skip(coords.len()) {
                    commands.entity(*child).despawn_recursive();
                }
                children.truncate(coords.len());
            }

            for (i, coord) in coords.iter().enumerate() {
                let mut transform = Transform::from_translation(*coord);
                transform.rotate(Quat::from_rotation_y(facing));
                if i < children.len() {
                    let child = children[i];
                    let mut scenery_element = q_scenery_elements.get_mut(child).unwrap();
                    if scenery_element.exemplar == *exemplar {
                        scenery_element.position = *coord;
                        scenery_element.facing = facing;
                        commands.entity(child).insert(transform);
                        continue;
                    }
                    commands.entity(child).despawn_recursive();
                }

                let child = commands
                    .spawn((
                        SceneryElement {
                            iid: SceneryInstanceId::Internal(0),
                            exemplar: exemplar.clone(),
                            facing,
                            position: *coord,
                        },
                        SpatialBundle {
                            transform,
                            ..default()
                        },
                        RenderLayers::layer(r_drag_state.cursor_layer),
                        SceneryElementRebuildAspects,
                    ))
                    .id();
                children.push(child);
            }

            commands.entity(cursor_model).replace_children(&children);
        }
    }

    if !show_wall && r_drag_state.cursor_model.is_some() {
        commands
            .entity(r_drag_state.cursor_model.unwrap())
            .despawn_recursive();
        r_drag_state.cursor_model = None;
    }
}

#[derive(Debug, Event)]
struct UndoPlaceWalls {
    label: &'static str,
    precinct: Handle<PrecinctAsset>,
    added: Vec<SceneryInstanceData>,
    removed: Vec<SceneryInstanceData>,
}

impl UndoEntry for UndoPlaceWalls {
    fn label(&self) -> &'static str {
        self.label
    }

    fn undo(&self, world: &mut World) {
        let Some(mut precinct_assets) = world.get_resource_mut::<Assets<PrecinctAsset>>() else {
            return;
        };
        let Some(precinct) = precinct_assets.get_mut(self.precinct.id()) else {
            return;
        };
        let to_remove = HashSet::<SceneryInstanceId>::from_iter(
            self.added.iter().map(|scenery| scenery.iid.clone()),
        );
        precinct.remove_scenery_elements(|iid, _, _| to_remove.contains(iid));
        for scenery in self.removed.iter() {
            precinct.add_scenery_element(
                scenery.id,
                scenery.facing,
                scenery.position,
                Some(scenery.iid.clone()),
            );
        }
    }

    fn redo(&self, world: &mut World) {
        let Some(mut precinct_assets) = world.get_resource_mut::<Assets<PrecinctAsset>>() else {
            return;
        };
        let Some(precinct) = precinct_assets.get_mut(self.precinct.id()) else {
            return;
        };
        let to_remove = HashSet::<SceneryInstanceId>::from_iter(
            self.removed.iter().map(|scenery| scenery.iid.clone()),
        );
        precinct.remove_scenery_elements(|iid, _, _| to_remove.contains(iid));
        for scenery in self.added.iter() {
            precinct.add_scenery_element(
                scenery.id,
                scenery.facing,
                scenery.position,
                Some(scenery.iid.clone()),
            );
        }
    }
}

fn place_walls(
    trigger: Trigger<PlaceWalls>,
    mut r_precinct_assets: ResMut<Assets<PrecinctAsset>>,
    r_server: Res<AssetServer>,
    mut r_modified_precincts: ResMut<ModifiedAssets<PrecinctAsset>>,
    mut r_undo_stack: ResMut<UndoStack>,
) {
    let event = trigger.event();
    let precinct = match r_precinct_assets.get_mut(event.precinct.id()) {
        Some(precinct) => precinct,
        None => {
            r_precinct_assets.insert(event.precinct.id(), PrecinctAsset::default());
            r_precinct_assets.get_mut(event.precinct.id()).unwrap()
        }
    };
    let exemplar_path = r_server.get_path(event.exemplar).unwrap().to_string();
    let archetype_id = match precinct.scenery_type_index(&exemplar_path) {
        Some(id) => id,
        None => precinct.add_scenery_type(exemplar_path),
    };
    let height = event.tier as f32 + TIER_OFFSET;
    // TODO: For undo purposes, TBA
    let mut added: Vec<SceneryInstanceData> = Vec::new();
    let removed = precinct.remove_scenery_elements(|_, _, pos| {
        // TODO: Check for wall type?
        pos.x >= event.area.min.x
            && pos.x <= event.area.max.x
            && pos.y > height - 0.5
            && pos.y < height + 0.5
            && pos.z >= event.area.min.y
            && pos.z <= event.area.max.y
    });
    let mut x = event.area.min.x;
    while x <= event.area.max.x {
        let mut z = event.area.min.y;
        while z <= event.area.max.y {
            let position = Vec3::new(x, height, z);
            added.push(SceneryInstanceData {
                iid: precinct.add_scenery_element(archetype_id, event.facing, position, None),
                id: archetype_id,
                facing: event.facing,
                position,
                aspects: Default::default(),
            });
            z += 1.0;
        }
        x += 1.0;
    }
    r_modified_precincts.add(event.precinct.clone());
    r_undo_stack.push(UndoPlaceWalls {
        label: "Place Walls",
        precinct: event.precinct.clone(),
        added,
        removed,
    });
}

fn remove_walls(
    trigger: Trigger<RemoveWalls>,
    mut r_precinct_assets: ResMut<Assets<PrecinctAsset>>,
    mut r_modified_precincts: ResMut<ModifiedAssets<PrecinctAsset>>,
    mut r_undo_stack: ResMut<UndoStack>,
) {
    let event = trigger.event();
    let precinct = match r_precinct_assets.get_mut(event.precinct.id()) {
        Some(precinct) => precinct,
        None => {
            r_precinct_assets.insert(event.precinct.id(), PrecinctAsset::default());
            r_precinct_assets.get_mut(event.precinct.id()).unwrap()
        }
    };
    let height = event.tier as f32 + TIER_OFFSET;
    let removed = precinct.remove_scenery_elements(|_, _, pos| {
        // TODO: Check for wall type?
        pos.x >= event.area.min.x
            && pos.x <= event.area.max.x
            && pos.y > height - 0.5
            && pos.y < height + 0.5
            && pos.z >= event.area.min.y
            && pos.z <= event.area.max.y
    });
    // println!("Removed {} walls.", removed.len());
    r_modified_precincts.add(event.precinct.clone());
    r_undo_stack.push(UndoPlaceWalls {
        label: "Remove Walls",
        precinct: event.precinct.clone(),
        added: Vec::new(),
        removed,
    });
}
