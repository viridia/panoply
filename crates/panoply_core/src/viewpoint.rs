use crate::transition::Transition;
use crate::{PrimaryCamera, Realm};
use bevy::render::view::RenderLayers;
use bevy::{ecs::world::Command, prelude::*};
use std::f32::consts::PI;

/// Represents the focal point of attention, typically the coordinates of the player
/// character.
#[derive(Resource, Default, Debug)]
pub struct Viewpoint {
    pub realm: Option<Entity>,
    pub position: Vec3,
    pub azimuth: f32,
    pub elevation: f32,
    pub camera_distance: f32,
    pub camera_distance_transition: Option<Transition<f32>>,
}

impl Viewpoint {
    /// Move the viewpoint to a new position
    pub fn _move_to(&mut self, position: Vec3) -> &mut Self {
        self.position = position;
        self
    }

    /// Move the viewpoint by a relative amount.
    pub fn _move_rel(&mut self, position: Vec3) -> &mut Self {
        self.position += position;
        self
    }

    /// Relative viewpoint movement in local coordinates.
    pub fn move_local(&mut self, forward: f32, strafe: f32) -> &mut Self {
        self.position +=
            Quat::from_euler(EulerRot::ZYX, 0., self.azimuth, 0.) * Vec3::new(strafe, 0., forward);
        self
    }

    /// Rotate the viewpoint by a relative amount.
    pub fn rotate(&mut self, angle: f32) -> &mut Self {
        self.azimuth = (self.azimuth + angle).rem_euclid(PI * 2.);
        self
    }

    pub fn get_camera_transform(&self, transform: &mut Transform) {
        transform.rotation =
            Quat::from_euler(EulerRot::ZYX, 0., self.azimuth + PI, -self.elevation);
        transform.translation =
            self.position + transform.rotation * Vec3::new(0., 0., self.camera_distance);
    }

    pub fn camera_distance(&self) -> f32 {
        self.camera_distance
    }

    pub fn set_camera_distance(&mut self, distance: f32, duration: f32) {
        if duration <= 0. {
            self.camera_distance = distance;
            self.camera_distance_transition = None;
        } else {
            self.camera_distance_transition = Some(Transition::new(
                self.camera_distance,
                distance,
                duration,
                bevy::prelude::EaseFunction::QuadraticInOut,
            ));
        }
    }
}

pub fn viewpoint_transitions(time: Res<Time>, mut viewpoint: ResMut<Viewpoint>) {
    if let Some(transition) = &mut viewpoint.camera_distance_transition {
        transition.advance(time.delta_secs());
        let value = transition.current();
        let finished = transition.is_finished();
        viewpoint.camera_distance = value;
        if finished {
            viewpoint.camera_distance_transition = None;
        }
    }
}

/// Command which directly sets the viewpoint to a new position, used in editor.
/// For gameplay, the viewpoint follows the "active entity".
pub struct SetViewpointCmd {
    pub position: Vec3,
    pub realm: String,
}

impl Command for SetViewpointCmd {
    fn apply(self, world: &mut World) {
        println!("Go to {} {}", self.realm, self.position);
        let mut realms = world.query::<(Entity, &Realm)>();
        if let Some(realm) = realms
            .iter(world)
            .find(|(_, r)| r.name == self.realm)
            .map(|(e, _)| e)
        {
            let mut viewpoint = world.get_resource_mut::<Viewpoint>().unwrap();
            viewpoint.realm = Some(realm);
            viewpoint.position = self.position;
        }
    }
}

/// Synchronize the camera position with the viewpoint.
pub fn update_camera_pos(
    viewpoint: ResMut<Viewpoint>,
    mut q_camera: Query<(&mut Transform, &mut RenderLayers), With<PrimaryCamera>>,
    q_realms: Query<&Realm>,
) {
    let (mut transform, mut layers) = q_camera.single_mut();

    // Update the camera transform
    viewpoint.get_camera_transform(&mut transform);

    // Update the camera render layers
    match viewpoint.realm {
        Some(realm) => {
            if let Ok(realm) = q_realms.get(realm) {
                *layers = realm.layer.clone();
            } else {
                *layers = RenderLayers::none();
            }
        }
        None => {
            *layers = RenderLayers::none();
        }
    }
}

// import { ease, EaseFunction, invariant } from '@faery/common';
// import { createAtom } from '@faery/reflex';
// import { Box3, Vector3 } from 'three';

// /** A class which tracks where the camera is looking at, and what objects are visible. */
// export class Viewpoint {
//   #positionAtom = createAtom();
//   #positionEaseStart = new Vector3();
//   #positionEaseEnd = new Vector3();
//   #positionEaseFunction: EaseFunction = 'linear';
//   #positionEaseDuration: number = 0;
//   #positionEaseParam: number | null = null;
//   #getCameraDistance: Accessor<number>;
//   #setCameraDistance: Setter<number>;
//   #cutaways: Box3[] = [];
//   #cutawaysAtom = createAtom();

//   constructor(private engine: IEngine) {
//     this.beforeAnimate = this.beforeAnimate.bind(this);
//     [this.#getRealmId, this.#setRealmId] = createSignal('default');
//     [this.#getAzimuth, this.#setAzimuth] = createSignal(0);
//     [this.#getElevation, this.#setElevation] = createSignal(0);
//     [this.#getCameraDistance, this.#setCameraDistance] = createSignal(11);
//     this.engine.subscribe('beforeAnimate', this.beforeAnimate);
//     engine.addSystem(VIEWPOINT_KEY, this);
//   }

//   dispose() {
//     this.engine.unsubscribe('beforeAnimate', this.beforeAnimate);
//   }

//   /** Get the current realm object. Throw if not a valid realm object. */
//   public getActiveRealm(): Realm {
//     const realmName = this.#getRealmId();
//     const realm = this.engine.world.getRealm(realmName);
//     invariant(realm, `Invalid realm name: ${realmName}`);
//     return realm;
//   }

//   /** Get the current realm object, or undefined if current realm does not exist. */
//   public maybeGetActiveRealm(): Realm | undefined {
//     const realmName = this.#getRealmId();
//     const world = this.engine.world;
//     return world.getRealm(realmName);
//   }

//   /** The current view position (where the camera is looking at). */
//   public get position(): Readonly<Vector3> {
//     this.#positionAtom.onObserved();
//     return this.#position;
//   }

//   /** Move the camera to a new position, and possibly a different realm. */
//   public moveTo(position: Vector3, realm?: string): void {
//     if (realm) {
//       this.#setRealmId(realm);
//     }
//     if (!this.#position.equals(position)) {
//       this.#positionEaseParam = null;
//       this.#position.copy(position);
//       this.#positionAtom.onChanged();
//     }
//   }

//   /** Add a displacement vector to the viewpoint. */
//   public moveRelative(v: Vector3): void {
//     this.#positionEaseParam = null;
//     this.#position.add(v);
//     this.#positionAtom.onChanged();
//   }

//   /** Move the camera to a new position, given by coordinates. */
//   public setPosition(x: number, y: number, z: number): void {
//     this.#positionEaseParam = null;
//     this.#position.set(x, y, z);
//     this.#positionAtom.onChanged();
//   }

//   /** Move the camera to a new position, and possibly a different realm. */
//   public easeTo(position: Vector3, duration: number, fn: EaseFunction): void {
//     this.#positionEaseStart.copy(this.#position);
//     this.#positionEaseEnd.copy(position);
//     this.#positionEaseParam = 0;
//     this.#positionEaseDuration = duration;
//     this.#positionEaseFunction = fn;
//   }

//   /** The list of cutaway volumes, which exclude scenery from being rendered.. */
//   public get cutaways(): Box3[] {
//     this.#cutawaysAtom.onObserved();
//     return this.#cutaways;
//   }

//   public set cutaways(locations: Box3[]) {
//     this.#cutaways = locations;
//     this.#cutawaysAtom.onChanged();
//   }

//   private beforeAnimate(delta: number) {
//     // Do position easing
//     if (typeof this.#positionEaseParam === 'number') {
//       const amount = delta / this.#positionEaseDuration;
//       this.#positionEaseParam = Math.min(1, this.#positionEaseParam + amount);
//       this.#position.lerpVectors(
//         this.#positionEaseStart,
//         this.#positionEaseEnd,
//         ease(this.#positionEaseParam, this.#positionEaseFunction)
//       );

//       if (this.#positionEaseParam >= 1) {
//         this.#positionEaseParam = null;
//       }

//       this.#positionAtom.onChanged();
//     }
//   }
// }

// export const VIEWPOINT_KEY = createSystemKey<Viewpoint>('Viewpoint');
