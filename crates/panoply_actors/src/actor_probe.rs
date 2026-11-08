//! The interaction probe: sweeps ahead of the player's actor for something it can interact with,
//! shows that interaction's caption as a prompt, and performs it on E.

use bevy::prelude::*;
use bevy_action_map::prelude::*;
use panoply_core::{
    realm_physics::collision_groups, RealmPhysics, ViewOverlay, GAME_UI_FONT, KEY_PROMPT_FONT,
};
use panoply_exemplar::{perform_interaction, Interactions};
use rapier3d::{
    parry::{query::details::ShapeCastOptions, shape::Cuboid},
    prelude::{Group, InteractionGroups, InteractionTestMode, Pose, QueryFilter},
};

use crate::{
    actor_control::{PlayerControl, PlayerInteract},
    actor_physics::capsule,
    Actor, ActorMotion, ActorScale, CollisionSize, PlayerActor,
};

/// How far the probe reaches from the actor's axis: the default interaction distance (R92).
const PROBE_REACH: f32 = 1.5;

/// Half the width and half the depth of the box the probe sweeps.
const PROBE_HALF_WIDTH: f32 = 0.1;
const PROBE_HALF_DEPTH: f32 = 0.02;

/// Distance above the bottom of the screen of the prompt, in pixels.
const PROMPT_BOTTOM: f32 = 24.;

/// The interaction the player could perform now.
#[derive(Clone, PartialEq)]
pub(crate) struct Probed {
    target: Entity,
    caption: String,
    action: String,
}

/// What the probe last found.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct ProbedInteraction(Option<Probed>);

/// The name of the control that performs the probed interaction, if it is bound.
#[derive(Resource, Default)]
pub(crate) struct InteractionKey(Option<String>);

/// The text showing the key that performs the probed interaction, empty when there is none. Its
/// child, [`InteractionPromptCaption`], continues the line in another font.
#[derive(Component)]
pub(crate) struct InteractionPrompt;

/// The span showing the probed interaction's caption.
#[derive(Component)]
pub(crate) struct InteractionPromptCaption;

/// Sweeps a thin box from the actor's feet to the top of its head along its facing, so that it
/// finds a chest on the ground and a sign at eye level alike, and offers the first enabled
/// interaction of what it hits first. Ground and floors would stop a box that reaches the feet, and
/// the canopy is passed through (D39). Actors are not yet colliders (D41).
pub(crate) fn probe_interaction(
    input: ContextActions<PlayerControl>,
    players: Query<
        (
            &Actor,
            &ActorMotion,
            Option<&CollisionSize>,
            Option<&ActorScale>,
        ),
        With<PlayerActor>,
    >,
    realms: Query<&RealmPhysics>,
    interactions: Query<&Interactions>,
    mut probed: ResMut<ProbedInteraction>,
    mut commands: Commands,
) {
    let found = players.single().ok().and_then(|player| {
        let (actor, motion, size, scale) = player;
        let world = realms.get(actor.realm).ok()?.world();
        let shape = capsule(size, scale);
        let half_height = shape.half_height() + shape.radius;
        let rotation = Quat::from_rotation_y(motion.facing);
        let filter = QueryFilter::default().groups(InteractionGroups::new(
            Group::ALL,
            !(collision_groups::CANOPY | collision_groups::TERRAIN | collision_groups::FLOOR),
            InteractionTestMode::And,
        ));
        let (handle, hit) = world.cast_shape(
            &Pose::from_parts(motion.position + Vec3::Y * half_height, rotation),
            rotation * Vec3::Z,
            &Cuboid::new(Vec3::new(PROBE_HALF_WIDTH, half_height, PROBE_HALF_DEPTH)),
            ShapeCastOptions::with_max_time_of_impact(PROBE_REACH),
            filter,
        )?;
        let target = Entity::try_from_bits(world.colliders[handle].user_data as u64)?;
        let entry = interactions
            .get(target)
            .ok()?
            .first_enabled(hit.time_of_impact)?;
        Some(Probed {
            target,
            caption: entry.caption.clone(),
            action: entry.action.clone(),
        })
    });
    if input.fired::<PlayerInteract>() {
        if let Some(Probed { target, action, .. }) = found.clone() {
            commands.queue(move |world: &mut World| perform_interaction(world, target, &action));
        }
    }
    probed.set_if_neq(ProbedInteraction(found));
}

/// Forgets the probed interaction on leaving play, when the probe stops.
pub(crate) fn clear_probed_interaction(mut probed: ResMut<ProbedInteraction>) {
    probed.set_if_neq(ProbedInteraction(None));
}

/// Looks up the name of the control bound to [`PlayerInteract`]. Run when a binding changes.
pub(crate) fn update_interaction_key(world: &mut World) {
    let key = BindingTable::new(world)
        .prompts(
            ContextId::of::<PlayerControl>(),
            PlayerInteract::id(),
            PromptScope::ANY,
        )
        .first()
        .map(|prompt| prompt.origin.fallback_label().into_owned());
    world.resource_mut::<InteractionKey>().0 = key;
}

pub(crate) fn spawn_interaction_prompt(mut commands: Commands) {
    let font = |family: &str| TextFont {
        font: FontSource::Family(family.into()),
        font_size: FontSize::Px(20.),
        ..default()
    };
    commands.spawn((
        Name::new("InteractionPrompt"),
        InteractionPrompt,
        ViewOverlay,
        Text::new(""),
        font(KEY_PROMPT_FONT),
        children![(
            Name::new("InteractionPromptCaption"),
            InteractionPromptCaption,
            TextSpan::default(),
            font(GAME_UI_FONT),
            TextColor(Color::WHITE),
        )],
        TextLayout::justify(Justify::Center),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(PROMPT_BOTTOM),
            left: Val::Px(0.),
            right: Val::Px(0.),
            ..default()
        },
    ));
}

pub(crate) fn update_interaction_prompt(
    probed: Res<ProbedInteraction>,
    key: Res<InteractionKey>,
    mut key_text: Single<&mut Text, With<InteractionPrompt>>,
    mut caption: Single<&mut TextSpan, With<InteractionPromptCaption>>,
) {
    if !probed.is_changed() && !key.is_changed() {
        return;
    }
    key_text.0 = match (&probed.0, &key.0) {
        (Some(_), Some(key)) => format!("[{key}] "),
        _ => String::new(),
    };
    caption.0 = probed
        .0
        .as_ref()
        .map_or_else(String::new, |found| found.caption.clone());
}
