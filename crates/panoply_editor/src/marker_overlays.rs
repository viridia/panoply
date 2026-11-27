//! Scenery mode's marker overlays, for every marker fixture of the shown precincts, as faery's: the
//! fixture's map pin, tinted by its [`MarkerPin`]; a [`MarkerArea`]'s fill and outline; and a
//! [`MarkerPath`]'s points as pins, joined by a dashed line. Faint, and bright while the fixture is
//! selected.
//!
//! A marker's overlays hang from a root of their own, not from the fixture, whose children are its
//! models and are despawned with them. The root follows the fixture's place but not its facing,
//! which an area and a path ignore, and is despawned with it.

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    transform::TransformSystems,
};
use panoply_core::PrimaryCamera;
use panoply_overlays::{
    AddOverlay, DrawPlane, OverlayColor, OverlayShape, PolygonOptions, ShapeBuilder,
};
use panoply_scenery::{MarkerArea, MarkerAreaShape, MarkerPath, MarkerPin};

use crate::{instance_selection::ScenerySelection, mode_selector::EditorMode};

/// The pin's size, in meters, as faery drew it.
const PIN_SIZE: Vec2 = Vec2::new(0.25, 0.4);

/// The point of the pin's image at the fixture, as a fraction of its size from its bottom left:
/// below the image, so the tip stands just above the fixture.
const PIN_ANCHOR: Vec2 = Vec2::new(0.5, -0.1);

/// How much a pin's tint is darkened while faint and lightened while bright, in HSL lightness.
const PIN_SHADE: f32 = 0.2;

/// A path's points, faint and bright.
const POINT_COLORS: [Srgba; 2] = [
    Srgba::rgb(0.133, 0.667, 0.133),
    Srgba::rgb(0.533, 1., 0.267),
];

/// An area's outline's opacity, faint and bright.
const OUTLINE_ALPHA: [f32; 2] = [0.2, 1.];

/// An area's fill's opacity, faint and bright: enough to show the area, not to hide the ground.
const FILL_ALPHA: [f32; 2] = [0.05, 0.25];

/// A path's line's opacity, faint and bright.
const ROUTE_ALPHA: [f32; 2] = [0.3, 0.7];

/// How far above the fixture an area or a path is drawn, so flat ground there does not hide it.
const LIFT: f32 = 0.02;

const OUTLINE_WIDTH: f32 = 0.08;
const ROUTE_WIDTH: f32 = 0.09;
const DASH: f32 = 0.2;
const GAP: f32 = 0.15;

/// The overlays of a marker fixture: a root, under which they are drawn.
#[derive(Component)]
#[relationship(relationship_target = MarkerOverlays)]
struct MarkerOverlayOf(Entity);

/// A marker fixture's overlay root, despawned with it.
#[derive(Component)]
#[relationship_target(relationship = MarkerOverlayOf, linked_spawn)]
struct MarkerOverlays(Vec<Entity>);

/// A marker's overlay root, and whether its fixture is selected, which draws it bright.
#[derive(Component, Reflect, Clone, Copy, PartialEq, Default)]
#[reflect(Component)]
pub struct MarkerOverlay {
    pub bright: bool,
}

/// A part of a marker's overlay, and its color faint and bright: an [`OverlayColor`]'s base, or a
/// pin's tint.
#[derive(Component, Clone, Copy)]
struct MarkerTint([Color; 2]);

/// A pin, turned to face the camera.
#[derive(Component)]
struct MarkerPinQuad;

/// A marker's shapes, around its fixture, in the world's axes.
#[derive(Component, Clone, PartialEq)]
enum MarkerFigure {
    AreaFill(MarkerAreaShape),
    AreaOutline(MarkerAreaShape),
    /// A path's points, joined by a dashed line.
    Route(Vec<Vec3>),
}

impl OverlayShape for MarkerFigure {
    fn draw(&self, builder: &mut ShapeBuilder) {
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(LIFT)
            .with_stroke_width(OUTLINE_WIDTH);
        match self {
            MarkerFigure::AreaFill(MarkerAreaShape::Circle { radius }) => {
                builder.fill_circle(Vec2::ZERO, *radius, circle_segments(*radius));
            }
            MarkerFigure::AreaFill(MarkerAreaShape::Rect { half_x, half_z }) => {
                builder.fill_rect(Rect::from_center_half_size(
                    Vec2::ZERO,
                    Vec2::new(*half_x, *half_z),
                ));
            }
            MarkerFigure::AreaOutline(MarkerAreaShape::Circle { radius }) => {
                builder.stroke_circle(Vec2::ZERO, *radius, circle_segments(*radius));
            }
            MarkerFigure::AreaOutline(MarkerAreaShape::Rect { half_x, half_z }) => {
                builder.stroke_rect(Rect::from_center_half_size(
                    Vec2::ZERO,
                    Vec2::new(*half_x, *half_z),
                ));
            }
            MarkerFigure::Route(points) => {
                builder.with_stroke_width(ROUTE_WIDTH).stroke_polyline_3d(
                    points,
                    PolygonOptions {
                        dash_length: DASH,
                        gap_length: GAP,
                        ..default()
                    },
                );
            }
        }
    }
}

/// Segments around a circle of `radius`: about one every 20 cm, enough to look round.
fn circle_segments(radius: f32) -> u32 {
    ((radius * 30.) as u32).clamp(24, 256)
}

/// The pin's quad and image, which every pin shares.
#[derive(Resource)]
struct PinAssets {
    mesh: Handle<Mesh>,
    image: Handle<Image>,
}

impl FromWorld for PinAssets {
    fn from_world(world: &mut World) -> Self {
        // The quad's middle, from the fixture: the anchor's offset from the image's middle.
        let middle = (Vec2::splat(0.5) - PIN_ANCHOR) * PIN_SIZE;
        let mesh = Rectangle::from_size(PIN_SIZE)
            .mesh()
            .build()
            .translated_by(middle.extend(0.));
        Self {
            mesh: world.resource_mut::<Assets<Mesh>>().add(mesh),
            image: world.resource::<AssetServer>().load("editor/map-pin.png"),
        }
    }
}

pub struct MarkerOverlaysPlugin;

impl Plugin for MarkerOverlaysPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MarkerOverlay>()
            .init_resource::<PinAssets>()
            .add_overlay::<MarkerFigure>()
            .add_systems(
                Update,
                (sync_marker_overlays, follow_markers)
                    .chain()
                    .run_if(in_state(EditorMode::Scenery)),
            )
            .add_systems(
                PostUpdate,
                face_camera
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(EditorMode::Scenery)),
            )
            .add_systems(OnExit(EditorMode::Scenery), despawn_marker_overlays);
    }
}

/// Gives each marker fixture its overlays, and draws them again when its marker aspects change:
/// attached, edited or detached.
#[allow(clippy::type_complexity)]
fn sync_marker_overlays(
    markers: Query<(
        Entity,
        Ref<MarkerPin>,
        Option<Ref<MarkerArea>>,
        Option<Ref<MarkerPath>>,
        &GlobalTransform,
        &RenderLayers,
        Has<MarkerOverlays>,
    )>,
    unmarked: Query<Entity, (With<MarkerOverlays>, Without<MarkerPin>)>,
    mut removed_areas: RemovedComponents<MarkerArea>,
    mut removed_paths: RemovedComponents<MarkerPath>,
    selection: Res<ScenerySelection>,
    pins: Res<PinAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for entity in &unmarked {
        commands.entity(entity).despawn_related::<MarkerOverlays>();
    }
    let detached: Vec<Entity> = removed_areas.read().chain(removed_paths.read()).collect();
    for (entity, pin, area, path, transform, layers, drawn) in &markers {
        let changed = pin.is_changed()
            || area.as_ref().is_some_and(Ref::is_changed)
            || path.as_ref().is_some_and(Ref::is_changed)
            || detached.contains(&entity);
        if drawn && !changed {
            continue;
        }
        if drawn {
            commands.entity(entity).despawn_related::<MarkerOverlays>();
        }
        let bright = selection.instance == Some(entity);
        let color: Color = pin.color.0.into();
        let pin_tint = MarkerTint([shaded(color, -PIN_SHADE), shaded(color, PIN_SHADE)]);
        let mut pin_quad = |offset: Vec3, tint: MarkerTint| {
            let material = materials.add(StandardMaterial {
                base_color: tint.0[bright as usize],
                base_color_texture: Some(pins.image.clone()),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                fog_enabled: false,
                cull_mode: None,
                ..default()
            });
            (
                Name::new("MarkerPin"),
                MarkerPinQuad,
                tint,
                Mesh3d(pins.mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(offset),
                NotShadowCaster,
                NotShadowReceiver,
            )
        };
        let figure = |name: &'static str, figure: MarkerFigure, alpha: [f32; 2]| {
            let tint = MarkerTint(alpha.map(|a| color.with_alpha(a)));
            (
                Name::new(name),
                figure,
                tint,
                OverlayColor {
                    base: tint.0[bright as usize],
                    underlay: 0.3,
                },
                Transform::default(),
            )
        };
        let mut root = commands.spawn((
            Name::new("MarkerOverlay"),
            MarkerOverlay { bright },
            MarkerOverlayOf(entity),
            Transform::from_translation(transform.translation()),
            Visibility::default(),
            Propagate(layers.clone()),
        ));
        root.with_child(pin_quad(Vec3::ZERO, pin_tint));
        if let Some(area) = &area {
            root.with_child(figure(
                "MarkerAreaFill",
                MarkerFigure::AreaFill(area.shape.clone()),
                FILL_ALPHA,
            ));
            root.with_child(figure(
                "MarkerAreaOutline",
                MarkerFigure::AreaOutline(area.shape.clone()),
                OUTLINE_ALPHA,
            ));
        }
        if let Some(path) = &path {
            let points = MarkerTint(POINT_COLORS.map(Color::from));
            for &point in &path.points {
                root.with_child(pin_quad(point, points));
            }
            if path.points.len() > 1 {
                root.with_child(figure(
                    "MarkerRoute",
                    MarkerFigure::Route(path.points.clone()),
                    ROUTE_ALPHA,
                ));
            }
        }
    }
}

/// `color`, its HSL lightness raised by `amount`, or lowered if negative.
fn shaded(color: Color, amount: f32) -> Color {
    let hsla = Hsla::from(color);
    hsla.with_lightness((hsla.lightness + amount).clamp(0., 1.))
        .into()
}

/// Keeps each marker's overlays on its fixture, and bright while it is selected.
fn follow_markers(
    selection: Res<ScenerySelection>,
    fixtures: Query<&GlobalTransform>,
    mut roots: Query<(
        &MarkerOverlayOf,
        &mut MarkerOverlay,
        &mut Transform,
        &Children,
    )>,
    mut parts: Query<(
        &MarkerTint,
        Option<&mut OverlayColor>,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (of, mut overlay, mut transform, children) in &mut roots {
        let Ok(fixture) = fixtures.get(of.0) else {
            continue;
        };
        if transform.translation != fixture.translation() {
            transform.translation = fixture.translation();
        }
        let bright = selection.instance == Some(of.0);
        if overlay.bright == bright {
            continue;
        }
        overlay.bright = bright;
        let mut parts = parts.iter_many_mut(children);
        while let Some(part) = parts.fetch_next() {
            let Ok((tint, color, material)) = part else {
                continue;
            };
            let tint = tint.0[bright as usize];
            if let Some(mut color) = color {
                color.base = tint;
            }
            if let Some(mut material) = material.and_then(|m| materials.get_mut(m)) {
                material.base_color = tint;
            }
        }
    }
}

/// Turns each pin to face the camera, as a sprite does, its top toward the top of the view.
fn face_camera(
    cameras: Query<&Transform, (With<PrimaryCamera>, Without<MarkerPinQuad>)>,
    mut pins: Query<&mut Transform, With<MarkerPinQuad>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    for mut pin in &mut pins {
        if pin.rotation != camera.rotation {
            pin.rotation = camera.rotation;
        }
    }
}

fn despawn_marker_overlays(roots: Query<Entity, With<MarkerOverlay>>, mut commands: Commands) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
