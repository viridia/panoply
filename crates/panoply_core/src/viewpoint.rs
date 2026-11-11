use crate::transition::Transition;
use crate::{PrimaryCamera, Realm};
use bevy::camera::visibility::RenderLayers;
use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder};
use bevy::prelude::*;
use std::f32::consts::PI;

/// Represents the focal point of attention, typically the coordinates of the player
/// character.
#[derive(Resource, Default, Debug)]
pub struct Viewpoint {
    pub realm: Option<Entity>,
    /// The active entity (R102), whose `Transform` the viewpoint follows. Moving the viewpoint
    /// any other way (`GotoLocation`, the free camera's keys) clears it.
    pub follow: Option<Entity>,
    /// Whether the next follow step lands on the entity rather than chasing it; see
    /// [`Viewpoint::follow_snapped`].
    pub follow_snap: bool,
    pub position: Vec3,
    pub azimuth: f32,
    pub elevation: f32,
    pub camera_distance: f32,
    pub camera_distance_transition: Option<Transition<f32>>,
}

impl Viewpoint {
    /// Follow `entity`, landing on it at once whatever the distance, as after a change of realm or
    /// a loaded game. The azimuth stays as it is.
    pub fn follow_snapped(&mut self, entity: Entity) -> &mut Self {
        self.follow = Some(entity);
        self.follow_snap = true;
        self
    }

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

/// Request to move the viewpoint to a position in a named realm. Written by the editor's quick
/// navigation, or over the Bevy Remote Protocol with `world.write_message` by test plans.
/// Fields left out over BRP take their defaults, so the optional ones can be omitted.
#[derive(Message, Reflect, Clone, Debug, Default)]
#[reflect(Message, Default)]
pub struct GotoLocation {
    /// Name of the realm, such as `"playground"`.
    pub realm: String,
    /// Position in the realm, in meters.
    pub position: Vec3,
    /// If set, the camera distance to use there, so a test can measure at a known zoom.
    pub camera_distance: Option<f32>,
    /// If set, the heading to face there, in radians; 0 faces +z, PI/2 faces +x.
    pub azimuth: Option<f32>,
}

pub fn goto_location(mut messages: MessageReader<GotoLocation>, mut commands: Commands) {
    for goto in messages.read() {
        commands.queue(SetViewpointCmd {
            position: goto.position,
            realm: goto.realm.clone(),
            camera_distance: goto.camera_distance,
            azimuth: goto.azimuth,
        });
    }
}

/// Command which directly sets the viewpoint to a new position, used in editor.
/// For gameplay, the viewpoint follows the "active entity". Everything changes at once: a new
/// camera distance seen at the old position would load parcels around a place never shown, whose
/// precincts may never load.
pub struct SetViewpointCmd {
    pub position: Vec3,
    pub realm: String,
    pub camera_distance: Option<f32>,
    pub azimuth: Option<f32>,
}

impl Command for SetViewpointCmd {
    type Out = ();

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
            viewpoint.follow = None;
            if let Some(distance) = self.camera_distance {
                viewpoint.set_camera_distance(distance, 0.);
            }
            if let Some(azimuth) = self.azimuth {
                viewpoint.azimuth = azimuth.rem_euclid(PI * 2.);
            }
        }
    }
}

/// How far above the followed entity's origin the viewpoint aims.
const FOLLOW_HEIGHT: f32 = 1.5;
/// How close the viewpoint must be to its aim before it stops chasing.
const FOLLOW_SLACK: f32 = 0.25;
/// How fast the viewpoint chases its aim, per meter beyond the slack, and at most.
const FOLLOW_RATE: f32 = 2.0;
const FOLLOW_MAX_SPEED: f32 = 10.0;
/// From this far away, the viewpoint jumps to its aim instead of chasing it.
const FOLLOW_JUMP: f32 = 20.0;

/// Moves the viewpoint after the entity it follows, as faery's camera did: it trails behind,
/// faster the further it is, so a fall or a start shows on screen before the camera catches up.
/// From far away (a new game, a teleport) it jumps there and faces the entity's way. Before
/// `update_camera_pos`, and after actors are drawn between their fixed ticks.
pub fn follow_active_entity(
    mut viewpoint: ResMut<Viewpoint>,
    transforms: Query<&Transform>,
    time: Res<Time>,
) {
    let Some(entity) = viewpoint.follow else {
        return;
    };
    let Ok(transform) = transforms.get(entity) else {
        viewpoint.follow = None;
        viewpoint.follow_snap = false;
        return;
    };
    let aim = transform.translation + Vec3::Y * FOLLOW_HEIGHT;
    let offset = aim - viewpoint.position;
    let distance = offset.length();
    if viewpoint.follow_snap {
        viewpoint.position = aim;
        viewpoint.follow_snap = false;
    } else if distance >= FOLLOW_JUMP {
        viewpoint.position = aim;
        let (facing, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
        viewpoint.azimuth = facing.rem_euclid(PI * 2.);
    } else if distance > FOLLOW_SLACK {
        // Capped, so a hitch does not throw the camera past its aim.
        let delta = time.delta_secs().min(0.1);
        let speed = ((distance - FOLLOW_SLACK) * FOLLOW_RATE).min(FOLLOW_MAX_SPEED);
        viewpoint.position += offset / distance * (speed * delta).min(distance);
    }
}

/// Synchronize the camera position with the viewpoint.
pub fn update_camera_pos(
    viewpoint: ResMut<Viewpoint>,
    mut q_camera: Query<(&mut Transform, &mut RenderLayers), With<PrimaryCamera>>,
    q_realms: Query<&Realm>,
) {
    let Ok((mut transform, mut layers)) = q_camera.single_mut() else {
        return;
    };

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

/// Fits the sun's shadow cascades to what the camera can see. The camera looks down from
/// `camera_distance` away, so visible ground lies roughly between 0.4x and 2.5x that distance;
/// one cascade over that range is sharper and cheaper than Bevy's default four, whose near
/// cascades cover nothing on screen. Refits when the distance changes (editor zoom).
pub fn fit_shadow_cascades(
    viewpoint: Res<Viewpoint>,
    mut lights: Query<&mut CascadeShadowConfig, With<DirectionalLight>>,
) {
    let distance = viewpoint.camera_distance();
    let wanted = CascadeShadowConfigBuilder {
        num_cascades: 1,
        minimum_distance: distance * 0.4,
        maximum_distance: distance * 2.5,
        first_cascade_far_bound: distance * 2.5,
        overlap_proportion: 0.2,
    }
    .build();
    for mut config in lights.iter_mut() {
        if config.bounds != wanted.bounds || config.minimum_distance != wanted.minimum_distance {
            *config = wanted.clone();
        }
    }
}
