use bevy::prelude::{EulerRot, Quat, Resource, Transform, Vec3};
use std::f32::consts::PI;

/// Represents the focal point of attention, typically the coordinates of the player
/// character.
#[derive(Resource, Default, Debug)]
pub struct Viewpoint {
    pub realm: i32,
    pub position: Vec3,
    pub azimuth: f32,
    pub elevation: f32,
    pub camera_distance: f32,
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
