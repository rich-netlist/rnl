//! The connected parts of a net as a level draws it: the parts its flylines join (13.7) and that a
//! viewer may label with the net's name. This is what a viewer that does not solve draws, not a
//! fact of the netlist: two wires of a net are one part where an end or a bend of one is an end or
//! a bend of the other as drawn, and a pin is in the part of a wire that ends or bends on it.

use std::collections::{HashMap, HashSet};

use crate::geom::Pt;

use super::obj::*;
use super::Interp;

/// One connected part of a net as drawn: wires that meet at their ends and bends, and the pins
/// they meet, or a pin that no wire meets.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Island {
    /// What a flyline may start or end at: the vertices of its wires as drawn, after their mode,
    /// their bends included, then its pins; no point repeats.
    pub points: Vec<Pt>,
    /// The segments of its wires as drawn, of length 0 included.
    pub segments: Vec<(Pt, Pt)>,
    /// Its pins, where they are drawn.
    pub pins: Vec<Pt>,
}

impl Island {
    /// Where a viewer labels this part with its net's name: of the middles of its segments and of
    /// its pins, the one nearest the centre of its points, the first of those as near.
    pub(crate) fn label_at(&self) -> Option<Pt> {
        let n = self.points.len() as f64;
        let centre = self.points.iter().fold((0.0, 0.0), |c, p| (c.0 + p.0 / n, c.1 + p.1 / n));
        let middles = self.segments.iter().filter(|(a, b)| a != b).map(|(a, b)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
        let mut best: Option<(f64, Pt)> = None;
        for p in middles.chain(self.pins.iter().copied()) {
            let d = square(p, centre);
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, p));
            }
        }
        best.map(|(_, p)| p).or_else(|| self.points.first().copied())
    }
}

/// What one drawn thing of a net adds to its part: a wire's vertices and segments, or a pin.
struct Piece {
    points: Vec<Pt>,
    segments: Vec<(Pt, Pt)>,
    pin: Option<Pt>,
    handles: Vec<ObjId>,
}

fn square(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
}

/// A point as drawn, to the millionth of a unit: two points with the same key are one.
fn key(p: Pt) -> (i64, i64) {
    ((p.0 * 1e6).round() as i64, (p.1 * 1e6).round() as i64)
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The lines that join `islands` with the fewest and shortest lines, by the distance in a
/// straight line between their points: a minimum spanning tree of the parts (the viewer's choice,
/// 13.7), the same lines as joining the nearest two points of different parts first while two
/// parts are apart, when no two such pairs are as near. Grown from the first part: each line
/// goes from a point of the parts joined already to the nearest point of another, the first of
/// those as near; time goes as the square of the number of points, and memory as that number.
pub(crate) fn spanning(islands: &[Island]) -> Vec<(Pt, Pt)> {
    let nodes: Vec<(usize, Pt)> =
        islands.iter().enumerate().flat_map(|(k, island)| island.points.iter().map(move |p| (k, *p))).collect();
    let mut joined = vec![false; islands.len()];
    // for each point, its square distance to the parts joined already, and the nearest point there
    let mut best: Vec<(f64, usize)> = vec![(f64::INFINITY, 0); nodes.len()];
    let mut out = Vec::new();
    let mut next = (!islands.is_empty()).then_some(0);
    while let Some(k) = next {
        joined[k] = true;
        for (j, &(_, pj)) in nodes.iter().enumerate().filter(|(_, (kj, _))| *kj == k) {
            for (i, &(ki, pi)) in nodes.iter().enumerate() {
                if !joined[ki] {
                    let d = square(pi, pj);
                    if d < best[i].0 {
                        best[i] = (d, j);
                    }
                }
            }
        }
        next = None;
        let mut nearest: Option<usize> = None;
        for (i, &(ki, _)) in nodes.iter().enumerate() {
            if !joined[ki] && nearest.is_none_or(|n| best[i].0 < best[n].0) {
                nearest = Some(i);
            }
        }
        if let Some(i) = nearest {
            out.push((nodes[best[i].1].1, nodes[i].1));
            next = Some(nodes[i].0);
        }
    }
    out
}

impl Interp<'_> {
    /// The connected parts of `net` as the scene draws it, each wire of the net and each pin on it
    /// that has a position in one of them, in the order of their first wire or pin. A wire some of
    /// whose points have no position is one piece, its runs joined by its own flylines
    /// ([`Interp::wire_runs`]). Two pieces are in one part where a vertex of one as drawn, an end
    /// or a bend after its mode, or a pin, is a vertex of the other: a wire that ends on the middle
    /// of another, or crosses it, is apart from it. `terms` is what `route_terms` gives.
    pub(crate) fn islands(
        &mut self,
        s: SceneId,
        net: ObjId,
        terms: &HashMap<ObjId, Vec<(ObjId, ObjId, u32)>>,
    ) -> Vec<Island> {
        let by_handle = self.follows_0_4(self.level_unit[self.scenes[s].level]);
        let mut pieces = Vec::new();
        for w in self.scenes[s].wires_by_net.get(&net).cloned().unwrap_or_default() {
            if !matches!(self.o(w), ObjData::Wire { failed: false, .. }) {
                continue;
            }
            let Some(runs) = self.wire_runs(s, w) else { continue };
            let handles = if by_handle {
                self.wire_shape(s, w)
                    .map(|shape| shape.points.into_iter().map(|point| self.wire_point_handle(s, point)).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let mut piece = Piece { points: Vec::new(), segments: Vec::new(), pin: None, handles };
            for run in runs {
                let drawn = run;
                piece.segments.extend(drawn.windows(2).map(|pair| (pair[0], pair[1])));
                piece.points.extend(drawn);
            }
            if !piece.points.is_empty() {
                pieces.push(piece);
            }
        }
        let mut seen = HashSet::new();
        for &(owner, t, index) in terms.get(&net).into_iter().flatten() {
            if !self.has_pin(s, t) || !seen.insert((owner, index)) {
                continue;
            }
            if let Some(p) = self.point_of(s, t) {
                let handles = if by_handle { vec![self.wire_point_handle(s, t)] } else { Vec::new() };
                pieces.push(Piece { points: vec![p], segments: Vec::new(), pin: Some(p), handles });
            }
        }
        // In 0.4 only shared objects connect. Equal coordinates, including bends inserted by
        // a drawing mode, make no connection (13.1). Keep the released drawing of older files.
        let mut parent: Vec<usize> = (0..pieces.len()).collect();
        let mut at: HashMap<(i64, i64), usize> = HashMap::new();
        let mut connected = HashMap::new();
        for (i, piece) in pieces.iter().enumerate() {
            let shared: Vec<usize> = if by_handle {
                piece.handles.iter().map(|handle| *connected.entry(*handle).or_insert(i)).collect()
            } else {
                piece.points.iter().map(|point| *at.entry(key(*point)).or_insert(i)).collect()
            };
            for first in shared {
                let (a, b) = (find(&mut parent, first), find(&mut parent, i));
                parent[a.max(b)] = a.min(b);
            }
        }
        let mut islands: Vec<Island> = Vec::new();
        let mut keys: Vec<HashSet<(i64, i64)>> = Vec::new();
        let mut of_root: HashMap<usize, usize> = HashMap::new();
        for (i, piece) in pieces.into_iter().enumerate() {
            let root = find(&mut parent, i);
            let k = *of_root.entry(root).or_insert_with(|| {
                islands.push(Island::default());
                keys.push(HashSet::new());
                islands.len() - 1
            });
            let island = &mut islands[k];
            island.segments.extend(piece.segments);
            match piece.pin {
                Some(p) => island.pins.push(p),
                None => island.points.extend(piece.points.into_iter().filter(|p| keys[k].insert(key(*p)))),
            }
        }
        // the pins after the vertices of the wires
        for (island, keys) in islands.iter_mut().zip(&mut keys) {
            let pins: Vec<Pt> = island.pins.iter().copied().filter(|p| keys.insert(key(*p))).collect();
            island.points.extend(pins);
        }
        islands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn island(points: &[Pt]) -> Island {
        Island { points: points.to_vec(), ..Island::default() }
    }

    #[test]
    fn the_parts_are_joined_by_their_nearest_points_in_a_straight_line() {
        // from the first part, (7, -2) is nearer to (10, 1) than (10, 6) in a straight line,
        // though not along the axes; the third part is nearest to the second
        let parts = [island(&[(0.0, -1.0), (7.0, -2.0), (10.0, 6.0)]), island(&[(10.0, 1.0)]), island(&[(12.0, 1.0)])];
        assert_eq!(spanning(&parts), [((7.0, -2.0), (10.0, 1.0)), ((10.0, 1.0), (12.0, 1.0))]);
        assert_eq!(spanning(&parts[..1]), []);
        assert_eq!(spanning(&[]), []);
    }

    #[test]
    fn a_label_is_at_the_middle_or_pin_nearest_the_centre() {
        // an L of two segments and a pin at its end: the centre is nearer the middle of the long one
        let island = Island {
            points: vec![(0.0, 0.0), (6.0, 0.0), (6.0, 2.0)],
            segments: vec![((0.0, 0.0), (6.0, 0.0)), ((6.0, 0.0), (6.0, 2.0))],
            pins: vec![(0.0, 0.0)],
        };
        assert_eq!(island.label_at(), Some((3.0, 0.0)));
        let pin = Island { points: vec![(1.0, 2.0)], segments: Vec::new(), pins: vec![(1.0, 2.0)] };
        assert_eq!(pin.label_at(), Some((1.0, 2.0)));
    }
}
