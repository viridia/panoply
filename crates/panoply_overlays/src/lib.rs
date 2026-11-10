//! Overlays: shapes drawn over the scene, such as an editor tool's grid or cursor, as retained
//! meshes rebuilt only when what they show changes. An overlay is a component implementing
//! [`OverlayShape`], holding just what its shape depends on; whatever reads the model writes it
//! only on a difference, and the mesh follows. Where the scene hides an overlay it still shows,
//! fainter ([`OverlayColor::underlay`]).
//!
//! Forked from `bevy_reactor_gizmoids` without its reactive scope.

mod material;
mod shape_builder;

use bevy::{
    asset::{embedded_asset, RenderAssetUsages},
    ecs::component::Mutable,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::PrimitiveTopology,
    prelude::*,
};

use crate::material::{OverlayMaterial, UnderlayMaterial};
pub use crate::{
    material::OverlayColor,
    shape_builder::{DrawPlane, PolygonOptions, ShapeBuilder, StrokeMarker},
};

/// A shape drawn as an overlay, rebuilt when the component changes.
pub trait OverlayShape: Component<Mutability = Mutable> + PartialEq {
    fn draw(&self, builder: &mut ShapeBuilder);
}

/// Adds overlay shapes of type `S`.
pub trait AddOverlay {
    fn add_overlay<S: OverlayShape>(&mut self) -> &mut Self;
}

impl AddOverlay for App {
    fn add_overlay<S: OverlayShape>(&mut self) -> &mut Self {
        self.register_required_components::<S, OverlayColor>()
            .register_required_components::<S, NotShadowCaster>()
            .register_required_components::<S, NotShadowReceiver>()
            .add_observer(add_materials::<S>)
            .add_systems(PostUpdate, rebuild_mesh::<S>.before(sync_underlay))
    }
}

pub struct OverlaysPlugin;

impl Plugin for OverlaysPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/overlay.wesl");
        app.add_plugins((
            MaterialPlugin::<OverlayMaterial>::default(),
            MaterialPlugin::<UnderlayMaterial>::default(),
        ))
        .add_systems(PostUpdate, sync_underlay);
    }
}

/// The child that draws the hidden parts with the parent's mesh.
#[derive(Component)]
struct Underlay;

/// Gives a new overlay its material, and a child with the underlay's.
fn add_materials<S: OverlayShape>(
    add: On<Add<S>>,
    mut overlays: ResMut<Assets<OverlayMaterial>>,
    mut underlays: ResMut<Assets<UnderlayMaterial>>,
    mut commands: Commands,
) {
    let overlay = overlays.add(OverlayMaterial::default());
    let underlay = underlays.add(UnderlayMaterial::default());
    commands
        .entity(add.entity)
        .insert(MeshMaterial3d(overlay))
        .with_child((
            Underlay,
            MeshMaterial3d(underlay),
            NotShadowCaster,
            NotShadowReceiver,
        ));
}

/// Redraws each overlay whose shape changed, in place once it has a mesh.
fn rebuild_mesh<S: OverlayShape>(
    shapes: Query<(Entity, &S, Option<&Mesh3d>), Changed<S>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for (entity, shape, mesh) in &shapes {
        let mut builder = ShapeBuilder::new();
        shape.draw(&mut builder);
        if let Some(mesh) = mesh.filter(|mesh| meshes.contains(&mesh.0)) {
            builder.build(&mut meshes.get_mut(&mesh.0).unwrap());
            continue;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        builder.build(&mut mesh);
        commands.entity(entity).insert(Mesh3d(meshes.add(mesh)));
    }
}

/// Gives each underlay its parent's mesh, and both materials the overlay's color.
fn sync_underlay(
    overlays: Query<(&Mesh3d, Ref<OverlayColor>, &MeshMaterial3d<OverlayMaterial>)>,
    underlays: Query<
        (
            Entity,
            Option<&Mesh3d>,
            &MeshMaterial3d<UnderlayMaterial>,
            &ChildOf,
        ),
        With<Underlay>,
    >,
    mut overlay_materials: ResMut<Assets<OverlayMaterial>>,
    mut underlay_materials: ResMut<Assets<UnderlayMaterial>>,
    mut commands: Commands,
) {
    for (entity, mesh, underlay, child_of) in &underlays {
        let Ok((parent_mesh, color, overlay)) = overlays.get(child_of.parent()) else {
            continue;
        };
        if mesh != Some(parent_mesh) {
            commands.entity(entity).insert(parent_mesh.clone());
        }
        if color.is_changed() {
            if let Some(mut material) = overlay_materials.get_mut(overlay) {
                material.color = color.base.into();
            }
            if let Some(mut material) = underlay_materials.get_mut(underlay) {
                let alpha = color.base.alpha() * color.underlay;
                material.color = color.base.with_alpha(alpha).into();
            }
        }
    }
}
