//! Facts that contradict themselves (standard 8.5, as the draft 0.3 makes it clear for every
//! version, and as 9.5 of 0.2 shows). Before the facts of a level are resolved, each is checked on
//! its own: one that has no solution on its own fails with an error, takes no part in the
//! resolution and is not given to a solver; the rest of its statement, and of the usage it is
//! expanded in, stays. Only constraints can contradict themselves: a coordinate of PLACE, an
//! offset of TEXT and an ORIENT always have a solution on their own.
//!
//! The check reads what a constraint is written on, not where its members are: the points it
//! relates are free, and only what is written ties them. A member stands at a point, or at the
//! centre of the points of a set (14.1), which lies among them; two members with the same point,
//! or the centres of the same points, are at one place. So ALIGN and DISTRIBUTE always have a
//! solution on their own, all members at one coordinate. ORDER has none when two of its members
//! are at one place. DISTANCE has none when its two sides overlap whatever the positions, because
//! a place is on both or a centre is on one side and the points it is the centre of on the other,
//! and `min` is above 0; nor when it measures a place from itself and its bounds leave 0 out.
//! ORIENT-RELATION has none when an object is on both sides and the relation turns it. What ties
//! points through a symbol, such as two terminals of one instance, is not looked at: the check
//! never fails a fact that has a solution on its own.

use crate::geom::{fmt_num, Orient};

use super::obj::*;
use super::Interp;

/// Why a constraint has no solution on its own, and the section of its kind.
pub(crate) struct Contradiction {
    pub why: String,
    pub section: &'static str,
}

/// Where a member of a constraint stands: a point, or the centre of the distinct points of a set,
/// in the order of the objects (14.1). Two members that stand at the same are at one place.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    Point(ObjId),
    Centre(Vec<ObjId>),
}

/// Does a DISTANCE whose sides overlap whatever the positions have no solution? Its measure is
/// then at most 0, and exactly 0 when each side is one place, the same. The bounds within `tol` of
/// 0 count as 0 (11.7). Gives the bound that leaves the measure out.
pub(crate) fn distance_without_room(
    only_that_place: bool,
    min: Option<f64>,
    max: Option<f64>,
    tol: f64,
) -> Option<(&'static str, f64)> {
    if let Some(m) = min.filter(|&m| m > tol) {
        return Some(("min", m));
    }
    max.filter(|&m| only_that_place && m < -tol).map(|m| ("max", m))
}

impl Interp<'_> {
    /// The point a member stands at: the object, whose `::position` is the object itself (6.5).
    /// The `::position` of a set, its centre, is a point of its own.
    fn point_key(&self, m: ObjId) -> ObjId {
        match self.o(m) {
            ObjData::Pos { parent } if !self.is_set(*parent) => self.point_key(*parent),
            _ => m,
        }
    }

    /// The distinct points of an object or of a set, nested sets down to their innermost
    /// members (14.1), in the order of the objects.
    fn points_of(&mut self, s: SceneId, m: ObjId) -> Vec<ObjId> {
        let leaves = if self.is_set(m) { self.flatten(s, m, 7, None) } else { vec![m] };
        let mut out: Vec<ObjId> = leaves.into_iter().map(|l| self.point_key(l)).collect();
        out.sort();
        out.dedup();
        out
    }

    /// Where a member stands: a set at the centre of its points, as its `::position` does, and a
    /// set of one point at that point.
    fn place_of(&mut self, s: SceneId, m: ObjId) -> Place {
        self.place_within(s, m, 0)
    }

    /// [`Interp::place_of`], through at most a few centres of centres: a set that holds only its
    /// own centre has no place of its own.
    fn place_within(&mut self, s: SceneId, m: ObjId, depth: usize) -> Place {
        if depth > 8 {
            return Place::Point(m);
        }
        if let ObjData::Pos { parent } = self.o(m) {
            let parent = *parent;
            return self.place_within(s, parent, depth + 1);
        }
        if !self.is_set(m) {
            return Place::Point(m);
        }
        let points = self.points_of(s, m);
        match points.as_slice() {
            [one] => match self.o(*one) {
                ObjData::Pos { .. } => self.place_within(s, *one, depth + 1),
                _ => Place::Point(*one),
            },
            _ => Place::Centre(points),
        }
    }

    /// Why a constraint of scene `s` has no solution on its own (8.5), if it has none. `members`
    /// are the members of an ALIGN, ORDER or DISTRIBUTE, a set written `{…}` by its centre (14.1).
    pub(crate) fn self_contradiction(
        &mut self,
        s: SceneId,
        cd: &ConstraintData,
        members: &[ObjId],
    ) -> Option<Contradiction> {
        match cd.kind {
            // all the members at one coordinate hold them
            ConstraintKind::Align | ConstraintKind::Distribute => None,
            ConstraintKind::Order => self.order_contradiction(s, members),
            ConstraintKind::Distance => self.distance_contradiction(s, cd),
            ConstraintKind::OrientRelation => {
                // an object on both sides would have its own orientation turned by the relation,
                // which only R0 leaves as it is (14.9)
                let relation = cd.params.relation.filter(|r| *r != Orient::R0)?;
                let mut sides = Vec::new();
                for &a in &cd.args {
                    sides.push(if self.is_set(a) { self.flatten(s, a, 7, None) } else { vec![a] });
                }
                let (Some(from), Some(to)) = (sides.first(), sides.get(1)) else { return None };
                let shared = from.iter().copied().find(|m| to.contains(m))?;
                let why = format!(
                    "{} is on both of its sides, and its orientation cannot be its own turned by {}",
                    self.label(shared),
                    relation.as_str()
                );
                Some(Contradiction { why, section: "14.9" })
            }
        }
    }

    /// Two members of an ORDER at one place cannot be in strict order, either way (14.3).
    fn order_contradiction(&mut self, s: SceneId, members: &[ObjId]) -> Option<Contradiction> {
        let mut seen: Vec<(Place, ObjId)> = Vec::new();
        for &m in members {
            let place = self.place_of(s, m);
            if place == Place::Centre(Vec::new()) {
                continue; // an empty set is no member (14.1)
            }
            if let Some(&(_, first)) = seen.iter().find(|(p, _)| *p == place) {
                let why = if first == m {
                    format!("{} is a member twice", self.label(m))
                } else {
                    format!("{} and {} are at one place", self.label(first), self.label(m))
                };
                return Some(Contradiction {
                    why: format!("{why}, and a strict order cannot hold one place twice"),
                    section: "14.3",
                });
            }
            seen.push((place, m));
        }
        None
    }

    /// The two sides of a DISTANCE overlap whatever the positions when a place is on both, or a
    /// centre is on one side and all the points it is the centre of on the other: its measure is
    /// then at most 0, and exactly 0 when each side is that one place (14.5).
    fn distance_contradiction(&mut self, s: SceneId, cd: &ConstraintData) -> Option<Contradiction> {
        let sides = [self.points_of(s, cd.args[0]), self.points_of(s, cd.args[1])];
        let mut places: [Vec<(Place, ObjId)>; 2] = [Vec::new(), Vec::new()];
        for (k, side) in sides.iter().enumerate() {
            for &p in side {
                let place = self.place_of(s, p);
                places[k].push((place, p));
            }
        }
        let mut tie = None;
        'found: for (pa, a) in &places[0] {
            for (pb, b) in &places[1] {
                if pa == pb {
                    tie = Some(if a == b {
                        format!("{} is on both of its sides", self.label(*a))
                    } else {
                        format!("{} and {}, one on each side, are at one place", self.label(*a), self.label(*b))
                    });
                    break 'found;
                }
            }
        }
        if tie.is_none() {
            // a centre on one side, and the points it is the centre of on the other
            for (k, side) in places.iter().enumerate() {
                let other = &sides[1 - k];
                let holds = |p: &Place| matches!(p, Place::Centre(pts) if !pts.is_empty() && pts.iter().all(|q| other.contains(q)));
                if let Some((_, c)) = side.iter().find(|(p, _)| holds(p)) {
                    tie = Some(format!(
                        "{} is on one side, and the points it is the centre of on the other",
                        self.label(*c)
                    ));
                    break;
                }
            }
        }
        let tie = tie?;
        let only = places[0].len() == 1 && places[1].len() == 1 && places[0][0].0 == places[1][0].0;
        let (bound, value) = distance_without_room(only, cd.params.min, cd.params.max, self.tol)?;
        let why = if only {
            let (a, b) = (places[0][0].1, places[1][0].1);
            let to = if a == b { "itself".to_string() } else { self.label(b) };
            format!(
                "it is a distance from {} to {to}, which is 0, and {bound}={} leaves 0 out",
                self.label(a),
                fmt_num(value)
            )
        } else {
            format!("{tie}, so the distance between them is at most 0, below {bound}={}", fmt_num(value))
        };
        Some(Contradiction { why, section: "14.5" })
    }
}

#[cfg(test)]
mod tests {
    use super::distance_without_room;

    #[test]
    fn a_distance_whose_sides_overlap() {
        let tol = 1e-6;
        // at most 0: a min above 0 leaves it no room, within the tolerance of 0 counts as 0
        assert_eq!(distance_without_room(false, Some(4.0), None, tol), Some(("min", 4.0)));
        assert_eq!(distance_without_room(false, Some(1e-7), None, tol), None);
        assert_eq!(distance_without_room(false, Some(0.0), Some(2.0), tol), None);
        assert_eq!(distance_without_room(false, None, Some(-3.0), tol), None);
        // exactly 0, from a place to itself: a negative max leaves it out too
        assert_eq!(distance_without_room(true, None, Some(-3.0), tol), Some(("max", -3.0)));
        assert_eq!(distance_without_room(true, Some(-2.0), Some(-1.0), tol), Some(("max", -1.0)));
        assert_eq!(distance_without_room(true, Some(0.0), Some(0.0), tol), None);
        assert_eq!(distance_without_room(true, Some(-1.0), Some(1.0), tol), None);
    }
}
