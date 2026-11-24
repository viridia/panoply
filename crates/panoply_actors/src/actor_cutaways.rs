//! The player's cutaways (R15): in play, the scenery over the player's actor and between it and the
//! camera is cut away, actors on those floors with it.

use bevy::prelude::*;
use panoply_core::{GameMode, PrimaryCamera};
use panoply_exemplar::InstanceKind;
use panoply_scenery::{
    cutaways::{choose_cutaways, shown, Cutaways, TierRects},
    precinct::{Precinct, PrecinctKey, PrecinctTier},
    PrecinctCache, PRECINCT_SIZE_F,
};

use crate::{Actor, HomePrecinct, PlayerActor};

/// Chooses the cutaways from the rects of the precinct the player's actor is in; none outside play.
pub(crate) fn choose_player_cutaways(
    mode: Res<State<GameMode>>,
    players: Query<(&Actor, &GlobalTransform), With<PlayerActor>>,
    cameras: Query<&GlobalTransform, With<PrimaryCamera>>,
    cache: Res<PrecinctCache>,
    precincts: Query<(&Precinct, &Children)>,
    tiers: Query<&PrecinctTier>,
    mut cutaways: ResMut<Cutaways>,
) {
    let chosen = (|| {
        if *mode.get() != GameMode::Play {
            return None;
        }
        let (actor, transform) = players.single().ok()?;
        let camera = cameras.single().ok()?.translation();
        let position = transform.translation();
        let key = PrecinctKey {
            realm: actor.realm,
            x: (position.x / PRECINCT_SIZE_F).floor() as i32,
            z: (position.z / PRECINCT_SIZE_F).floor() as i32,
        };
        let (precinct, children) = precincts.get(cache.peek(&key)?).ok()?;
        let tiers: Vec<TierRects> = tiers
            .iter_many(children)
            .flatten()
            .map(|tier| TierRects {
                level: tier.level,
                rects: &tier.cutaways,
            })
            .collect();
        let origin = (precinct.coords.as_vec2() * PRECINCT_SIZE_F)
            .extend(0.)
            .xzy();
        Some(choose_cutaways(origin, &tiers, position, camera))
    })()
    .unwrap_or_default();
    if cutaways.chosen != chosen {
        cutaways.chosen = chosen;
    }
}

/// Hides the actors precincts place, and the player's, that stand in a cutaway.
pub(crate) fn hide_cut_away_actors(
    cutaways: Res<Cutaways>,
    mut actors: Query<
        (&GlobalTransform, &mut Visibility),
        (With<Actor>, Or<(With<HomePrecinct>, With<PlayerActor>)>),
    >,
) {
    for (transform, mut visibility) in &mut actors {
        visibility.set_if_neq(shown(
            cutaways.hides_point(transform.translation(), Some(InstanceKind::Actor)),
        ));
    }
}
