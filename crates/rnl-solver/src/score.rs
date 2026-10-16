//! The layered score of a routed candidate: each layer the weighted sum of its objectives, layers
//! compared lexicographically within their tolerances, ties by the canonical code (plan 5.8).
//!
//! The objectives are measured by the score functions of their table (`crate::objectives`) on
//! the candidate as it will be drawn: the positions and orientations the coordinate LP gave it
//! (`crate::coords`) and its wires, those routed for the ROUTEs (`crate::route`) and those written
//! (13.1), drawn by their mode. Before routing, the routing objectives measure the straight
//! connections of each net; that is the estimate the driver picks the candidates to route by.
//!
//! **The order of candidates** ([`order`]) has two counts ahead of the objectives, as the solver's
//! own rules rank after every fact and before every objective (8.5, 11.7): first the pairs of
//! symbols that overlap, then the joins that routing left undone, a ROUTE (13.7) being a request
//! of the file and the objectives only preferences of the solver. Among candidates equal in both,
//! the layers decide as `crate::objectives::Active::rank` does: layer by layer, the candidates
//! within the layer's tolerance of its least value kept, the canonical code breaking what is left
//! tied. An objective that is off has no layer, so it neither ranks nor breaks ties.

use rnl_runtime::problem::LevelProblem;

use crate::objectives::{Active, Routed, View};
use crate::rational::Q;
use crate::resolve::Geometry;
use crate::route::{written_path, Routing};
use crate::structure::Structure;

/// A candidate as the order of candidates sees it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    /// The pairs of symbols that overlap.
    pub overlaps: usize,
    /// The joins routing left undone (`Routing::joins_left`); 0 before routing.
    pub joins_left: usize,
    /// The value of each layer of the objectives in force.
    pub layers: Vec<Q>,
    /// The canonical code (`crate::search::Candidate::code`).
    pub code: Vec<u32>,
}

/// The wires a routed candidate is drawn with, as the routing objectives measure them: the wires
/// routed, each with its net, and the wires written, drawn by their mode. A wire written on a
/// net that a ROUTE asks for counts as of that net, so that the routed wires that meet it do not
/// count as crossing another net.
pub(crate) fn drawn(problem: &LevelProblem, s: &Structure, g: &Geometry, routing: &Routing) -> Routed {
    let drawn = &s.connections.drawn;
    // a net by the first entry drawn with its name
    let net = |d: usize| drawn.iter().position(|e| drawn.get(d).is_some_and(|x| x.net == e.net)).unwrap_or(d);
    let mut paths: Vec<(usize, Vec<[Q; 2]>)> =
        routing.drawn.iter().zip(&routing.paths).map(|(d, path)| (net(*d), path.clone())).collect();
    let first_written = drawn.len().saturating_sub(problem.wires.len());
    for w in 0..problem.wires.len() {
        if let Some(path) = written_path(problem, g, w) {
            paths.push((net(first_written + w), path));
        }
    }
    Routed { paths }
}

/// The value of each layer of the objectives in force on a placed candidate: with `routed`, the
/// routing objectives measure its wires; without, the straight connections of each net.
pub(crate) fn layers(s: &Structure, origins: &[[Q; 2]], g: &Geometry, routed: Option<&Routed>) -> Vec<Q> {
    let view = View { s, at: origins, orient: &g.orient };
    s.active.score(&view, routed)
}

/// The order of some candidates, best first (see the module): the fewest overlaps, then the fewest
/// joins left undone, then the layers within their tolerances, then the code.
pub(crate) fn order(active: &Active, entries: &[Entry]) -> Vec<usize> {
    let mut keys: Vec<(usize, usize)> = entries.iter().map(|e| (e.overlaps, e.joins_left)).collect();
    keys.sort_unstable();
    keys.dedup();
    let mut out = Vec::with_capacity(entries.len());
    for key in keys {
        let members: Vec<usize> =
            (0..entries.len()).filter(|&i| (entries[i].overlaps, entries[i].joins_left) == key).collect();
        let refs: Vec<(&[Q], &[u32])> =
            members.iter().map(|&i| (entries[i].layers.as_slice(), entries[i].code.as_slice())).collect();
        out.extend(active.rank(&refs).into_iter().map(|j| members[j]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objectives::Inputs;
    use crate::rational::int;
    use crate::{Layer, Objectives};

    fn entry(overlaps: usize, joins_left: usize, layers: &[i64], code: u32) -> Entry {
        Entry { overlaps, joins_left, layers: layers.iter().map(|v| int(*v)).collect(), code: vec![code] }
    }

    #[test]
    fn overlaps_then_joins_left_then_the_layers_then_the_code() {
        let active = Active::new(
            &Objectives { layers: vec![Layer::single("area"), Layer::single("aspect")] },
            &Inputs::default(),
        );
        let entries = [
            entry(1, 0, &[0, 0], 0),
            entry(0, 2, &[0, 0], 1),
            entry(0, 0, &[5, 0], 2),
            entry(0, 0, &[3, 9], 3),
            entry(0, 0, &[3, 1], 5),
            entry(0, 0, &[3, 1], 4),
            entry(0, 1, &[0, 0], 6),
        ];
        assert_eq!(order(&active, &entries), vec![5, 4, 3, 2, 6, 1, 0]);
    }

    /// Within a layer's tolerance the next layer decides, and an objective that is off breaks no
    /// tie: with no objective at all, the code alone orders candidates equal in the counts.
    #[test]
    fn tolerances_and_objectives_off() {
        let active = Active::new(
            &Objectives {
                layers: vec![Layer { items: vec![("area".to_string(), 1)], tolerance: 2.0 }, Layer::single("aspect")],
            },
            &Inputs::default(),
        );
        let entries = [entry(0, 0, &[1, 7], 0), entry(0, 0, &[2, 3], 1), entry(0, 0, &[4, 0], 2)];
        assert_eq!(order(&active, &entries), vec![1, 0, 2]);
        let none = Active::new(&Objectives::none(), &Inputs::default());
        let entries = [entry(0, 0, &[], 3), entry(0, 0, &[], 1), entry(0, 1, &[], 0), entry(0, 0, &[], 2)];
        assert_eq!(order(&none, &entries), vec![1, 3, 0, 2]);
    }
}
