//! Cutaways: scenery hidden so that it does not hide the player. Each tier of a precinct has
//! authored rects; the player's choose some of them (`choose_cutaways`), and the editor adds its
//! own volumes. Whatever a volume holds is hidden with `Visibility::Hidden`, so its descendants
//! are too, and picking passes through it: scenery elements and actors by their origin, floor
//! regions by their outline at their height.

use bevy::prelude::*;
use panoply_exemplar::{Exemplar, InstanceType};

use crate::{
    floor_region::FloorRegion, precinct::Precinct, scenery_element::SceneryElement, FLOOR_TYPE,
    TIER_OFFSET,
};

/// How far a rect's volume reaches past it: a meter each way, so that the walls on its edges are
/// inside, and a little below the tier, so that its own floor is.
const VOLUME_MARGIN: Vec3 = Vec3::new(1.0, 0.2, 1.0);

/// The first tier whose rects the player can choose is `floor(y - TIER_OFFSET + HEAD_HEIGHT)` for
/// feet at `y`, as in the three.js version: the lowest whose floor is 0.7 m to 1.7 m above them.
const HEAD_HEIGHT: f32 = 1.7;

/// A box in the world that hides what is in it, all of it, or only instances of one type.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CutawayVolume {
    pub min: Vec3,
    pub max: Vec3,
    /// Hides only instances of this type, if any.
    pub only: Option<InstanceType>,
}

impl CutawayVolume {
    fn hides(&self, kind: InstanceType) -> bool {
        self.only.is_none_or(|only| only == kind)
    }

    fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    fn intersects(&self, min: Vec3, max: Vec3) -> bool {
        min.cmple(self.max).all() && max.cmpge(self.min).all()
    }
}

/// The volumes hiding scenery: those the player's position chose, and the editor's. Changing it
/// tests everything again, so a writer leaves it alone when its volumes are the same.
#[derive(Resource, Default, Debug)]
pub struct Cutaways {
    pub chosen: Vec<CutawayVolume>,
    pub editor: Vec<CutawayVolume>,
}

impl Cutaways {
    fn volumes(&self) -> impl Iterator<Item = &CutawayVolume> {
        self.chosen.iter().chain(&self.editor)
    }

    /// Whether an instance of `kind` at `point` is hidden.
    pub fn hides_point(&self, point: Vec3, kind: InstanceType) -> bool {
        self.volumes()
            .any(|volume| volume.hides(kind) && volume.contains(point))
    }

    /// Whether an instance of `kind` filling the box from `min` to `max` is hidden.
    pub fn hides_box(&self, min: Vec3, max: Vec3, kind: InstanceType) -> bool {
        self.volumes()
            .any(|volume| volume.hides(kind) && volume.intersects(min, max))
    }
}

/// The tiers a precinct's player chooses from: each tier's level and its rects, in the precinct's
/// coordinates.
pub struct TierRects<'a> {
    pub level: i32,
    pub rects: &'a [Rect],
}

/// The volumes that keep the player in sight, chosen from the rects of the precinct at `origin`:
/// on each tier from the first over the player ([`HEAD_HEIGHT`]) up, the rects crossed by the triangle
/// between the ray straight up from `position` and the line to `camera`, then every rect on those
/// tiers that overlaps one chosen, until no more do. A tier's floor at height `h` cuts the triangle
/// in a segment, from under the player toward the camera, as far as the line to the camera has
/// risen to `h`, or all the way where the camera is lower than that.
pub fn choose_cutaways(
    origin: Vec3,
    tiers: &[TierRects],
    position: Vec3,
    camera: Vec3,
) -> Vec<CutawayVolume> {
    let first = (position.y - TIER_OFFSET + HEAD_HEIGHT).floor() as i32;
    let tiers: Vec<&TierRects> = tiers.iter().filter(|tier| tier.level >= first).collect();
    let start = position.xz() - origin.xz();
    let toward = camera.xz() - position.xz();
    let rise = camera.y - position.y;

    let mut chosen: Vec<(i32, Rect)> = Vec::new();
    for tier in &tiers {
        let height = tier.level as f32 + TIER_OFFSET;
        let reach = if rise > 0.0 {
            ((height - position.y) / rise).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let end = start + toward * reach;
        chosen.extend(
            tier.rects
                .iter()
                .filter(|rect| segment_meets_rect(start, end, **rect))
                .map(|rect| (tier.level, *rect)),
        );
    }

    let mut changed = true;
    while changed {
        changed = false;
        for tier in &tiers {
            for rect in tier.rects {
                let entry = (tier.level, *rect);
                if !chosen.contains(&entry) && chosen.iter().any(|(_, c)| rects_meet(*c, *rect)) {
                    chosen.push(entry);
                    changed = true;
                }
            }
        }
    }

    chosen
        .into_iter()
        .map(|(level, rect)| {
            let height = level as f32 + TIER_OFFSET;
            CutawayVolume {
                min: origin + Vec3::new(rect.min.x, height, rect.min.y) - VOLUME_MARGIN,
                max: Vec3::new(origin.x + rect.max.x, f32::INFINITY, origin.z + rect.max.y)
                    + VOLUME_MARGIN.with_y(0.0),
                only: None,
            }
        })
        .collect()
}

/// Whether two rects overlap or touch, so that rooms side by side are cut away together.
fn rects_meet(a: Rect, b: Rect) -> bool {
    a.min.cmple(b.max).all() && a.max.cmpge(b.min).all()
}

/// Whether the segment from `start` to `end` meets `rect`, edges included: the segment clipped to
/// each of the rect's slabs in turn.
fn segment_meets_rect(start: Vec2, end: Vec2, rect: Rect) -> bool {
    let delta = end - start;
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for axis in 0..2 {
        let (from, step) = (start[axis], delta[axis]);
        let (min, max) = (rect.min[axis], rect.max[axis]);
        if step == 0.0 {
            if from < min || from > max {
                return false;
            }
            continue;
        }
        let (a, b) = ((min - from) / step, (max - from) / step);
        low = low.max(a.min(b));
        high = high.min(a.max(b));
        if low > high {
            return false;
        }
    }
    true
}

/// Shows or hides each scenery element in a precinct, and each floor region, by the cutaways:
/// all of them when the cutaways change, otherwise those added or moved. Elements elsewhere, such
/// as the editor's previews, are left alone.
pub fn hide_cut_away_scenery(
    cutaways: Res<Cutaways>,
    mut elements: Query<
        (
            Ref<SceneryElement>,
            Ref<GlobalTransform>,
            &ChildOf,
            &mut Visibility,
        ),
        Without<FloorRegion>,
    >,
    mut floors: Query<(Ref<FloorRegion>, Ref<GlobalTransform>, &mut Visibility)>,
    precincts: Query<(), With<Precinct>>,
    exemplars: Res<Assets<Exemplar>>,
) {
    let all = cutaways.is_changed();
    for (element, transform, parent, mut visibility) in &mut elements {
        if !(all || element.is_added() || transform.is_changed())
            || !precincts.contains(parent.parent())
        {
            continue;
        }
        let kind = exemplars
            .get(&element.exemplar)
            .map_or(InstanceType::NONE, |exemplar| exemplar.0.meta_type);
        visibility.set_if_neq(shown(cutaways.hides_point(transform.translation(), kind)));
    }
    for (floor, transform, mut visibility) in &mut floors {
        if !(all || floor.is_changed() || transform.is_changed()) {
            continue;
        }
        let outline = floor
            .poly
            .iter()
            .fold(Rect::EMPTY, |rect, point| rect.union_point(*point));
        if outline.is_empty() {
            continue;
        }
        // Just under the floor's surface, as the three.js version tested a tier's floor.
        let base = transform.translation() + Vec3::Y * (floor.level as f32 + TIER_OFFSET);
        let min = base + Vec3::new(outline.min.x, -0.1, outline.min.y);
        let max = base + Vec3::new(outline.max.x, -0.05, outline.max.y);
        visibility.set_if_neq(shown(cutaways.hides_box(min, max, FLOOR_TYPE)));
    }
}

/// The visibility of something hidden or not.
pub fn shown(hidden: bool) -> Visibility {
    if hidden {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, z0: f32, x1: f32, z1: f32) -> Rect {
        Rect::new(x0, z0, x1, z1)
    }

    #[test]
    fn a_segment_meets_a_rect_it_crosses_or_touches() {
        let room = rect(2.0, 2.0, 4.0, 4.0);
        assert!(segment_meets_rect(Vec2::ZERO, Vec2::splat(5.0), room));
        assert!(segment_meets_rect(Vec2::splat(3.0), Vec2::splat(3.0), room));
        assert!(segment_meets_rect(
            Vec2::new(0.0, 2.0),
            Vec2::new(5.0, 2.0),
            room
        ));
        assert!(!segment_meets_rect(Vec2::ZERO, Vec2::splat(1.9), room));
        assert!(!segment_meets_rect(
            Vec2::new(0.0, 5.0),
            Vec2::new(5.0, 5.0),
            room
        ));
    }

    #[test]
    fn the_rect_overhead_is_chosen_from_the_tier_above_the_head() {
        let ground = [rect(0.0, 0.0, 10.0, 10.0)];
        let upper = [rect(0.0, 0.0, 10.0, 10.0)];
        let tiers = [
            TierRects {
                level: 0,
                rects: &ground,
            },
            TierRects {
                level: 2,
                rects: &upper,
            },
        ];
        // Standing on the ground floor, camera straight overhead.
        let volumes = choose_cutaways(
            Vec3::ZERO,
            &tiers,
            Vec3::new(5.0, 0.02, 5.0),
            Vec3::new(5.0, 10.0, 5.0),
        );
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].min.y, 2.02 - 0.2);
    }

    #[test]
    fn the_line_to_the_camera_chooses_a_rect_in_front() {
        let porch = [rect(8.0, 0.0, 12.0, 4.0)];
        let tiers = [TierRects {
            level: 3,
            rects: &porch,
        }];
        // The porch roof at 3 m lies between the player at x 2 and the camera at x 14, 6 m up:
        // the line to the camera reaches 3 m half way, at x 8.
        let position = Vec3::new(2.0, 0.02, 2.0);
        let camera = Vec3::new(14.0, 6.02, 2.0);
        assert_eq!(
            choose_cutaways(Vec3::ZERO, &tiers, position, camera).len(),
            1
        );
        // With the camera on the other side, it does not.
        let behind = Vec3::new(-10.0, 6.02, 2.0);
        assert!(choose_cutaways(Vec3::ZERO, &tiers, position, behind).is_empty());
    }

    #[test]
    fn rects_touching_a_chosen_one_are_chosen_too() {
        let rooms = [
            rect(0.0, 0.0, 4.0, 4.0),
            rect(4.0, 0.0, 8.0, 4.0),
            rect(20.0, 0.0, 24.0, 4.0),
        ];
        let tiers = [TierRects {
            level: 2,
            rects: &rooms,
        }];
        let position = Vec3::new(2.0, 0.02, 2.0);
        let volumes = choose_cutaways(Vec3::ZERO, &tiers, position, position + Vec3::Y * 10.0);
        assert_eq!(volumes.len(), 2);
    }

    #[test]
    fn the_first_tier_chosen_from_is_at_least_0_7_m_up() {
        let ceiling = [rect(0.0, 0.0, 10.0, 10.0)];
        let tiers = [TierRects {
            level: 1,
            rects: &ceiling,
        }];
        let camera = Vec3::new(5.0, 10.0, 5.0);
        // Standing at 0, the first is floor(0 + 1.7) = 1: the tier at 1 m.
        let standing = Vec3::new(5.0, 0.02, 5.0);
        assert_eq!(
            choose_cutaways(Vec3::ZERO, &tiers, standing, camera).len(),
            1
        );
        // Standing at 0.5 m, on a raised floor, it is 2: the tier at 1 m is too low.
        let raised = Vec3::new(5.0, 0.52, 5.0);
        assert!(choose_cutaways(Vec3::ZERO, &tiers, raised, camera).is_empty());
    }
}
