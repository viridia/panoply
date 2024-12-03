use bevy::prelude::*;
use nalgebra::Vector3;
use rapier3d::prelude::*;

use crate::{Realm, Viewpoint};

// use crate::{view::Viewpoint, world::Realm};

#[derive(Component)]
pub struct RealmPhysics {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    integration_parameters: IntegrationParameters,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    physics_pipeline: PhysicsPipeline,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    gravity: Vector3<f32>,
}

impl Default for RealmPhysics {
    fn default() -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            physics_pipeline: PhysicsPipeline::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            gravity: Vector3::<f32>::new(0.0, -9.81, 0.0),
        }
    }
}

impl RealmPhysics {
    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    pub fn insert_body(&mut self, body: RigidBody) -> RigidBodyHandle {
        self.rigid_body_set.insert(body)
    }

    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.rigid_body_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }

    pub fn insert_collider(
        &mut self,
        handle: RigidBodyHandle,
        collider: Collider,
    ) -> ColliderHandle {
        self.collider_set
            .insert_with_parent(collider, handle, &mut self.rigid_body_set)
    }

    // pub fn insert_colliders(&mut self, handle: RigidBodyHandle, colliders: &[Collider]) {
    //     for collider in colliders {
    //         self.collider_set
    //             .insert_with_parent(collider, handle, &mut self.rigid_body_set);
    //     }
    // }

    pub fn remove_collider(&mut self, collider: ColliderHandle) {
        self.collider_set.remove(
            collider,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            false,
        );
    }
}

pub fn realm_physics_system(
    mut realm_query: Query<(&Realm, &mut RealmPhysics)>,
    viewpoint: Res<Viewpoint>,
    // mut query: Query<&mut Transform>,
) {
    let Some(realm_entity) = viewpoint.realm else {
        return;
    };

    if let Ok((_realm, mut realm_physics)) = realm_query.get_mut(realm_entity) {
        realm_physics.step();
    }
}
