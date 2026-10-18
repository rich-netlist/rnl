//! Configured exact scoring and deterministic tolerance-window ranking.
use crate::model::PointId;
use crate::options::{Direction, ObjectiveScope, Term};
#[cfg(test)]
use crate::rational::frac;
use crate::rational::{int, Q};
use crate::route::CLEARANCE;
use crate::structure::Structure;
use crate::Objectives;
use num_traits::{Signed, Zero};
use rnl_runtime::geom::Orient;
use std::cell::OnceCell;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Objective {
    CurrentDirection,
    GroundDirection,
    ComponentAlignment,
    Distribution,
    Symmetry,
    Turns,
    Crossings,
    WireLength,
    Area,
    Aspect,
    TerminalWireAlignment,
    SymbolOverlaps,
    WireShorts,
    WireOverlaps,
    LabelOverlaps,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    Group,
    Level,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub structural: bool,
}
impl Objective {
    #[cfg(test)]
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::CurrentDirection => "current_direction",
            Self::GroundDirection => "symbol_direction",
            Self::ComponentAlignment => "symbol_alignment",
            Self::Distribution => "symbol_distribution",
            Self::Symmetry => "symbol_symmetry",
            Self::Turns => "wire_turns",
            Self::Crossings => "wire_crossings",
            Self::WireLength => "wire_length",
            Self::Area => "area",
            Self::Aspect => "aspect",
            Self::TerminalWireAlignment => "pin_wire_alignment",
            Self::SymbolOverlaps => "symbol_overlaps",
            Self::WireShorts => "wire_shorts",
            Self::WireOverlaps => "wire_overlaps",
            Self::LabelOverlaps => "label_overlaps",
        }
    }
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "current_direction" => Self::CurrentDirection,
            "symbol_direction" => Self::GroundDirection,
            "symbol_alignment" => Self::ComponentAlignment,
            "symbol_distribution" => Self::Distribution,
            "symbol_symmetry" => Self::Symmetry,
            "wire_turns" => Self::Turns,
            "wire_crossings" => Self::Crossings,
            "wire_length" => Self::WireLength,
            "area" => Self::Area,
            "aspect" => Self::Aspect,
            "pin_wire_alignment" => Self::TerminalWireAlignment,
            "symbol_overlaps" => Self::SymbolOverlaps,
            "wire_shorts" => Self::WireShorts,
            "wire_overlaps" => Self::WireOverlaps,
            "label_overlaps" => Self::LabelOverlaps,
            _ => return None,
        })
    }
    pub(crate) fn entry(self) -> Entry {
        Entry {
            structural: matches!(
                self,
                Self::CurrentDirection
                    | Self::GroundDirection
                    | Self::ComponentAlignment
                    | Self::Distribution
                    | Self::Crossings
                    | Self::WireLength
                    | Self::Area
                    | Self::Aspect
            ),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Inputs {
    pub wires: bool,
    pub currents: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ActiveLayer {
    pub items: Vec<(Objective, Q)>,
    pub terms: Vec<Term>,
    pub tolerance: Q,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Active {
    pub layers: Vec<ActiveLayer>,
    pub inert: Vec<String>,
}
impl Active {
    pub(crate) fn new(objectives: &Objectives, inputs: &Inputs) -> Self {
        let mut layers = Vec::new();
        let mut inert = Vec::new();
        for layer in &objectives.layers {
            let mut terms = layer.items.clone();
            terms.sort_by_key(Term::key);
            for t in &terms {
                if t.kind == "current_direction" && !inputs.currents {
                    inert.push(
                        "current_direction is inactive: no marked terminal with an explicit pin direction".into(),
                    );
                }
                if matches!(
                    t.kind.as_str(),
                    "wire_turns" | "wire_crossings" | "wire_length" | "wire_shorts" | "wire_overlaps"
                ) && !inputs.wires
                {
                    inert.push(format!("{} is inactive: nothing drawn as wires", t.kind));
                }
            }
            let items =
                terms.iter().filter_map(|t| Objective::from_name(&t.kind).map(|o| (o, t.weight.clone()))).collect();
            layers.push(ActiveLayer { items, terms, tolerance: layer.tolerance.clone() });
        }
        Self { layers, inert }
    }
    #[cfg(test)]
    pub(crate) fn is_on(&self, o: Objective) -> bool {
        self.layers.iter().any(|l| l.items.iter().any(|(x, _)| *x == o))
    }
    pub(crate) fn in_order(&self) -> Vec<Objective> {
        self.layers
            .iter()
            .flat_map(|l| l.items.iter().map(|(o, _)| *o))
            .filter(|o| {
                *o != Objective::CurrentDirection || !self.inert.iter().any(|s| s.starts_with("current_direction"))
            })
            .collect()
    }
    pub(crate) fn terms(&self) -> impl Iterator<Item = &Term> {
        self.layers.iter().flat_map(|l| &l.terms)
    }
    pub(crate) fn report(
        &self,
        view: &View,
        routed: Option<&Routed>,
        options: &crate::SolverOptions,
    ) -> crate::ScoreReport {
        use crate::rational::exact_text;
        let config = options.canonical();
        let scoring = Scoring::new(*view, routed);
        let levels = self
            .layers
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let values: Vec<Q> = l.terms.iter().map(|t| scoring.term(t)).collect();
                let total: Q = l.terms.iter().zip(&values).map(|(t, v)| v * &t.weight).sum();
                let terms = l
                    .terms
                    .iter()
                    .zip(values)
                    .map(|(t, v)| {
                        let (inactive, diagnostics) = term_status(t, view);
                        crate::ScoreTerm {
                            kind: t.kind.clone(),
                            parameters: t.parameters(),
                            raw: exact_text(&v),
                            weight: exact_text(&t.weight),
                            weighted: exact_text(&(&v * &t.weight)),
                            estimated: routed.is_none() || t.kind == "label_overlaps",
                            inactive,
                            diagnostics,
                        }
                    })
                    .collect();
                crate::ScoreLevel { level: i, total: exact_text(&total), tolerance: exact_text(&l.tolerance), terms }
            })
            .collect();
        crate::ScoreReport { config_digest: rnl_runtime::problem::options_digest(&config), config, levels }
    }
    pub(crate) fn estimate(&self, view: &View) -> Vec<Q> {
        #[cfg(test)]
        let _profile = crate::test_profile::section("active.estimate");
        self.score(view, None)
    }
    pub(crate) fn score(&self, view: &View, routed: Option<&Routed>) -> Vec<Q> {
        let scoring = Scoring::new(*view, routed);
        let values = self.layers.iter().map(|l| l.terms.iter().map(|t| scoring.term(t) * &t.weight).sum()).collect();
        #[cfg(test)]
        if crate::test_profile::oracle_enabled() {
            let _profile = crate::test_profile::section("reference.score");
            let reference: Vec<Q> = self
                .layers
                .iter()
                .map(|l| l.terms.iter().map(|t| score_term(t, view, routed) * &t.weight).sum())
                .collect();
            assert_eq!(values, reference, "cached full exact score differs from reference");
        }
        values
    }
    pub(crate) fn rank(&self, entries: &[(&[Q], &[u32])]) -> Vec<usize> {
        let by_code = |a: &usize, b: &usize| entries[*a].1.cmp(entries[*b].1).then(a.cmp(b));
        if self.layers.iter().all(|l| l.tolerance.is_zero()) {
            let mut order: Vec<usize> = (0..entries.len()).collect();
            order.sort_by(|a, b| {
                entries[*a]
                    .0
                    .iter()
                    .take(self.layers.len())
                    .cmp(entries[*b].0.iter().take(self.layers.len()))
                    .then_with(|| by_code(a, b))
            });
            return order;
        }
        let mut left: Vec<usize> = (0..entries.len()).collect();
        let mut order = Vec::with_capacity(entries.len());
        while !left.is_empty() {
            let mut pool = left.clone();
            for (i, layer) in self.layers.iter().enumerate() {
                if pool.len() < 2 {
                    break;
                }
                let Some(least) = pool.iter().filter_map(|&j| entries[j].0.get(i)).min().cloned() else { break };
                let bound = least + &layer.tolerance;
                pool.retain(|&j| entries[j].0.get(i).is_none_or(|v| *v <= bound));
            }
            let Some(best) = pool.into_iter().min_by(by_code) else { break };
            order.push(best);
            left.retain(|&j| j != best);
        }
        order
    }
}

#[derive(Clone, Copy)]
pub(crate) struct View<'a> {
    pub s: &'a Structure,
    pub at: &'a [[Q; 2]],
    pub orient: &'a [Orient],
}
impl View<'_> {
    fn bbox(&self, k: usize) -> Option<[Q; 4]> {
        self.s.box_at(self.at, self.orient, k)
    }
    fn point(&self, p: PointId) -> Option<[Q; 2]> {
        self.s.point_at(self.at, self.orient, p)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Routed {
    pub paths: Vec<(usize, Vec<[Q; 2]>)>,
    pub geometry: crate::routing_geometry::RoutingGeometry,
}

struct TreeConnection {
    net: usize,
    routed: bool,
    a: PointId,
    b: PointId,
    from: [Q; 2],
    to: [Q; 2],
}
/// A cache never survives a scoring call, even when only one orientation changes.
struct Scoring<'a> {
    geometry: crate::score::ViewGeometry<'a>,
    routed: Option<&'a Routed>,
    tree: OnceCell<Vec<TreeConnection>>,
    envelope: OnceCell<Option<[Q; 4]>>,
}
impl<'a> Scoring<'a> {
    fn new(view: View<'a>, routed: Option<&'a Routed>) -> Self {
        Self {
            geometry: crate::score::ViewGeometry::new(view),
            routed,
            tree: OnceCell::new(),
            envelope: OnceCell::new(),
        }
    }
    fn tree(&self) -> &[TreeConnection] {
        self.tree.get_or_init(|| {
            let mut out = Vec::new();
            for (net, drawn) in self.geometry.view.s.connections.drawn.iter().enumerate() {
                let points: Vec<_> = drawn.points.iter().filter_map(|p| Some((*p, self.geometry.point(*p)?))).collect();
                let positions: Vec<_> = points.iter().map(|(_, q)| q.clone()).collect();
                for (a, b) in spanning_tree(&positions) {
                    out.push(TreeConnection {
                        net,
                        routed: drawn.routed,
                        a: points[a].0,
                        b: points[b].0,
                        from: points[a].1.clone(),
                        to: points[b].1.clone(),
                    });
                }
            }
            out
        })
    }
    fn term(&self, t: &Term) -> Q {
        #[cfg(test)]
        let _profile = crate::test_profile::section(&t.kind);
        let view = &self.geometry.view;
        match t.kind.as_str() {
            "symbol_overlaps" => self.geometry.overlaps().into_q(),
            "label_overlaps" => self.geometry.texts_over().into_q(),
            "wire_shorts" | "wire_overlaps" => {
                self.geometry.wire_penalty(self.routed, t.kind == "wire_shorts").into_q()
            }
            "wire_length" if self.routed.is_none() => self.tree().iter().map(|c| manhattan(&c.from, &c.to)).sum(),
            "wire_turns" if self.routed.is_none() => {
                let exit = |p: PointId| match p {
                    PointId::Pin(k, pin) => view.s.exit(k, pin, view.orient[k]),
                    _ => None,
                };
                self.tree()
                    .iter()
                    .map(|c| {
                        let (a, b) = (&c.from, &c.to);
                        let (ea, eb) = (exit(c.a), exit(c.b));
                        int(if c.routed {
                            connection_turns(a, ea, b, eb)
                        } else {
                            usize::from(a[0] != b[0] && a[1] != b[1])
                                + usize::from(!leaves_towards(a, ea, b))
                                + usize::from(!leaves_towards(b, eb, a))
                        } as i64)
                    })
                    .sum()
            }
            "wire_crossings" if self.routed.is_none() => {
                let segments = self
                    .tree()
                    .iter()
                    .flat_map(|c| {
                        let middle = [c.to[0].clone(), c.from[1].clone()];
                        [(c.net, c.from.clone(), middle.clone()), (c.net, middle, c.to.clone())]
                    })
                    .collect::<Vec<_>>();
                crossings(&segments)
            }
            "area" | "aspect" => match self.envelope.get_or_init(|| {
                let mut out: Option<[Q; 4]> = None;
                for k in view.s.components() {
                    let Some(b) = self.geometry.bbox(k) else { continue };
                    out = Some(match out {
                        None => b.clone(),
                        Some(e) => [
                            e[0].clone().min(b[0].clone()),
                            e[1].clone().min(b[1].clone()),
                            e[2].clone().max(b[2].clone()),
                            e[3].clone().max(b[3].clone()),
                        ],
                    });
                }
                out
            }) {
                Some(e) => {
                    let (w, h) = (&e[2] - &e[0], &e[3] - &e[1]);
                    if t.kind == "area" {
                        w * h
                    } else {
                        aspect(&w, &h, &t.ratio)
                    }
                }
                None => Q::zero(),
            },
            _ => score_term(t, view, self.routed),
        }
    }
}
#[cfg(test)]
pub(crate) fn score(o: Objective, view: &View, routed: Option<&Routed>) -> Q {
    let mut term = Term::new(o.name());
    term.direction = Some(Direction::Down);
    score_term(&term, view, routed)
}
pub(crate) fn score_term(t: &Term, view: &View, routed: Option<&Routed>) -> Q {
    let scope = if t.scope == ObjectiveScope::Group { Scope::Group } else { Scope::Level };
    match t.kind.as_str() {
        "current_direction" => current_term(t, view),
        "symbol_direction" => symbol_term(t, view),
        "symbol_alignment" => scoped(view, scope, alignment),
        "symbol_distribution" => {
            scoped(view, scope, |p| gap_kinds(p.iter().map(|p| &p[0])) + gap_kinds(p.iter().map(|p| &p[1])))
        }
        "symbol_overlaps" => crate::coords::overlapping(view.s, view.at, view.orient).len().into_q(),
        "label_overlaps" => crate::score::texts_over(view.s, view.at, view.orient).into_q(),
        "wire_shorts" | "wire_overlaps" => crate::score::wire_penalty(view, routed, t.kind == "wire_shorts").into_q(),
        "wire_turns" => match routed {
            Some(r) => r.paths.iter().map(|(_, p)| int(bends(p) as i64)).sum(),
            None => turns(view),
        },
        "wire_crossings" => {
            let mut segments = Vec::new();
            if let Some(r) = routed {
                for (net, path) in &r.paths {
                    for w in path.windows(2) {
                        segments.push((*net, w[0].clone(), w[1].clone()));
                    }
                }
            } else {
                for (net, a, b) in connections(view) {
                    let c = [b[0].clone(), a[1].clone()];
                    segments.push((net, a, c.clone()));
                    segments.push((net, c, b));
                }
            }
            crossings(&segments)
        }
        "wire_length" => match routed {
            Some(r) => r.paths.iter().flat_map(|(_, p)| p.windows(2).map(|w| manhattan(&w[0], &w[1]))).sum(),
            None => connections(view).iter().map(|(_, a, b)| manhattan(a, b)).sum(),
        },
        "area" | "aspect" => match envelope(view) {
            Some(e) => {
                let (w, h) = (&e[2] - &e[0], &e[3] - &e[1]);
                if t.kind == "area" {
                    w * h
                } else {
                    aspect(&w, &h, &t.ratio)
                }
            }
            None => Q::zero(),
        },
        _ => Q::zero(),
    }
}
fn aspect(w: &Q, h: &Q, ratio: &Q) -> Q {
    let rh = ratio * h;
    let den = w + &rh;
    if den.is_zero() {
        Q::zero()
    } else {
        (w - rh).abs() / den
    }
}
fn current_term(t: &Term, view: &View) -> Q {
    use rnl_runtime::problem::CurrentDirection;
    let target = t.direction.unwrap_or(Direction::Down).vector();
    view.s
        .currents
        .iter()
        .filter(|c| {
            let Some(rnl_runtime::problem::PointRef::Pin { object, pin, .. }) = &c.display else { return false };
            if view.s.objects[*object].left_free || !view.s.explicit_pin(*object, *pin) {
                return false;
            }
            let Some(mut d) = view.s.exit(*object, *pin, view.orient[*object]) else { return false };
            if c.direction == CurrentDirection::In {
                d = (-d.0, -d.1)
            }
            d != target
        })
        .count()
        .into_q()
}
/// Symbol-direction matches all actually drawn objects, independently of search
/// components or collision-marker exemptions (phase0 reply B03).
fn symbol_direction_objects(s: &Structure) -> impl Iterator<Item = usize> + '_ {
    s.objects
        .iter()
        .enumerate()
        .filter_map(|(object, info)| (info.symbol.is_some() && !info.left_free).then_some(object))
}
fn symbol_term(t: &Term, view: &View) -> Q {
    let target = t.direction.unwrap_or(Direction::Down).vector();
    symbol_direction_objects(view.s)
        .filter(|&k| {
            let Some(symbol) = view.s.objects[k].symbol else { return false };
            let name = &view.s.symbol_names[symbol];
            if !t.symbols.contains(name) {
                return false;
            }
            let reference = t.references.get(name).copied().or(t.reference).map(Direction::vector).or_else(|| {
                let pins = view.s.pins(k);
                if pins.len() != 1 {
                    return None;
                }
                let b = &view.s.symbols[symbol].bbox;
                let dx = (&b[0] + &b[2]) / int(2) - &pins[0].local.0;
                let dy = (&b[1] + &b[3]) / int(2) - &pins[0].local.1;
                if dx.is_zero() && !dy.is_zero() {
                    Some((0, if dy.is_positive() { 1 } else { -1 }))
                } else if dy.is_zero() && !dx.is_zero() {
                    Some((if dx.is_positive() { 1 } else { -1 }, 0))
                } else {
                    None
                }
            });
            let Some(d) = reference else { return false };
            let local = (int(d.0), int(d.1));
            crate::model::turned(view.orient[k], &local, 0) != int(target.0)
                || crate::model::turned(view.orient[k], &local, 1) != int(target.1)
        })
        .count()
        .into_q()
}
fn term_status(t: &Term, view: &View) -> (bool, Vec<String>) {
    let mut diagnostics = Vec::new();
    let inactive = match t.kind.as_str() {
        "current_direction" => {
            let ports = view
                .s
                .currents
                .iter()
                .filter(|c| matches!(c.terminal, rnl_runtime::problem::CurrentTerminal::Port { .. }))
                .count();
            if ports > 0 {
                diagnostics.push(format!("{ports} local port terminals have no drawing pin in this version"));
            }
            !view.s.currents.iter().any(|c| match &c.display {
                Some(rnl_runtime::problem::PointRef::Pin { object, pin, .. }) => {
                    !view.s.objects[*object].left_free
                        && view.s.explicit_pin(*object, *pin)
                        && view.s.exit(*object, *pin, view.orient[*object]).is_some()
                }
                _ => false,
            })
        }
        "symbol_direction" => {
            let mut count = 0;
            for k in symbol_direction_objects(view.s) {
                let Some(symbol) = view.s.objects[k].symbol else { continue };
                let name = &view.s.symbol_names[symbol];
                if !t.symbols.contains(name) {
                    continue;
                }
                if t.references.contains_key(name) || t.reference.is_some() {
                    count += 1;
                    continue;
                }
                let pins = view.s.pins(k);
                let b = &view.s.symbols[symbol].bbox;
                let valid = pins.len() == 1 && {
                    let dx = &b[0] + &b[2] - &pins[0].local.0 * int(2);
                    let dy = &b[1] + &b[3] - &pins[0].local.1 * int(2);
                    dx.is_zero() != dy.is_zero()
                };
                if valid {
                    count += 1
                } else {
                    diagnostics.push(format!(
                        "symbol {} requires a cardinal reference direction",
                        view.s.problem.objects[k].label
                    ));
                }
            }
            count == 0
        }
        "wire_turns" | "wire_crossings" | "wire_length" | "wire_shorts" | "wire_overlaps" => {
            view.s.problem.wires.is_empty() && view.s.problem.routes.is_empty()
        }
        "symbol_alignment" | "symbol_distribution" if t.scope == ObjectiveScope::Group => view.s.scopes.is_empty(),
        _ => view.s.components().next().is_none(),
    };
    if inactive {
        diagnostics.push("this term has no eligible objects in this level".into());
    }
    (inactive, diagnostics)
}
trait IntoQ {
    fn into_q(self) -> Q;
}

impl IntoQ for usize {
    fn into_q(self) -> Q {
        int(self as i64)
    }
}

/// A measure of the components' origins: summed over the scopes of the groups (`Structure::scopes`)
/// for an objective measured within groups, none for the components in no group; over all of them
/// for one measured over the level.
fn scoped(view: &View, scope: Scope, measure: impl Fn(&[[Q; 2]]) -> Q) -> Q {
    let of = |ks: &mut dyn Iterator<Item = usize>| {
        let points: Vec<[Q; 2]> = ks.map(|k| view.at[k].clone()).collect();
        measure(&points)
    };
    match scope {
        Scope::Group => view.s.scopes.iter().map(|ks| of(&mut ks.iter().copied())).sum(),
        Scope::Level => of(&mut view.s.components()),
    }
}

/// Distinct x plus distinct y.
fn alignment(points: &[[Q; 2]]) -> Q {
    let xs: BTreeSet<&Q> = points.iter().map(|p| &p[0]).collect();
    let ys: BTreeSet<&Q> = points.iter().map(|p| &p[1]).collect();
    (xs.len() + ys.len()).into_q()
}

/// The prototype's gap kinds of one projection (its `regularity.gap_kinds`): the distinct
/// coordinates sorted, the gaps between neighbours, and the number of kinds of gaps less one,
/// plus `1 − Σ (nₖ / n)²` over the kinds, which is below 1 and prefers one dominant spacing. No
/// gap, no cost.
pub(crate) fn gap_kinds<'a>(coords: impl Iterator<Item = &'a Q>) -> Q {
    let xs: Vec<&Q> = coords.collect::<BTreeSet<&Q>>().into_iter().collect();
    if xs.len() < 2 {
        return Q::zero();
    }
    let mut counts: std::collections::BTreeMap<Q, i64> = std::collections::BTreeMap::new();
    for w in xs.windows(2) {
        *counts.entry(w[1] - w[0]).or_default() += 1;
    }
    int(counts.len() as i64 - 1)
}

/// Mirror pairs not symmetric: not on one row, orientations not mirrored (`b` is `a` then MY,
/// 14.9), or not equally far from their axis when they have one.
fn manhattan(a: &[Q; 2], b: &[Q; 2]) -> Q {
    (&a[0] - &b[0]).abs() + (&a[1] - &b[1]).abs()
}

/// The connections of every net drawn as wires: the edges of a minimum spanning tree of its
/// points by rectilinear distance (Prim's, from its first point, ties to the earlier point), each
/// with the index of its net.
fn connections(view: &View) -> Vec<(usize, [Q; 2], [Q; 2])> {
    let mut out = Vec::new();
    for (i, drawn) in view.s.connections.drawn.iter().enumerate() {
        let points: Vec<[Q; 2]> = drawn.points.iter().filter_map(|p| view.point(*p)).collect();
        for (a, b) in spanning_tree(&points) {
            out.push((i, points[a].clone(), points[b].clone()));
        }
    }
    out
}

/// The edges of a minimum spanning tree of points by rectilinear distance (Prim's): each joins a
/// point not yet in the tree, the first of the nearest, to the first nearest point in it.
pub(crate) fn spanning_tree(points: &[[Q; 2]]) -> Vec<(usize, usize)> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let mut inside = vec![false; n];
    inside[0] = true;
    let mut best: Vec<(Q, usize)> = (0..n).map(|j| (manhattan(&points[0], &points[j]), 0)).collect();
    let mut edges = Vec::new();
    for _ in 1..n {
        let mut pick: Option<usize> = None;
        for j in 0..n {
            if !inside[j] && pick.is_none_or(|p| best[j].0 < best[p].0) {
                pick = Some(j);
            }
        }
        let Some(j) = pick else { break };
        inside[j] = true;
        edges.push((best[j].1, j));
        for m in 0..n {
            if !inside[m] {
                let d = manhattan(&points[j], &points[m]);
                if d < best[m].0 {
                    best[m] = (d, j);
                }
            }
        }
    }
    edges
}

/// The turns of the connections drawn straight, with the direction each pin leaves (9.2.4): for a
/// net routed, as the router draws each ([`connection_turns`]); for a wire written, one for a
/// connection not along an axis and one at each end whose pin does not leave towards the other
/// end.
fn turns(view: &View) -> Q {
    let mut count = 0usize;
    for drawn in &view.s.connections.drawn {
        let points: Vec<(PointId, [Q; 2])> = drawn.points.iter().filter_map(|p| Some((*p, view.point(*p)?))).collect();
        let positions: Vec<[Q; 2]> = points.iter().map(|(_, q)| q.clone()).collect();
        let exit = |p: PointId| match p {
            PointId::Pin(k, pin) => view.s.exit(k, pin, view.orient[k]),
            _ => None,
        };
        for (a, b) in spanning_tree(&positions) {
            let (pa, qa) = &points[a];
            let (pb, qb) = &points[b];
            let (ea, eb) = (exit(*pa), exit(*pb));
            count += if drawn.routed {
                connection_turns(qa, ea, qb, eb)
            } else {
                usize::from(qa[0] != qb[0] && qa[1] != qb[1])
                    + usize::from(!leaves_towards(qa, ea, qb))
                    + usize::from(!leaves_towards(qb, eb, qa))
            };
        }
    }
    count.into_q()
}

/// Whether a pin at `from` that leaves by `exit` leaves towards `to`; a point that is no pin, or a
/// pin without a direction, leaves any way.
fn leaves_towards(from: &[Q; 2], exit: Option<(i64, i64)>, to: &[Q; 2]) -> bool {
    let Some((x, y)) = exit else { return true };
    let dot = int(x) * (&to[0] - &from[0]) + int(y) * (&to[1] - &from[1]);
    dot.is_positive()
}

/// How the end of a connection leaves towards its other end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Leaving {
    /// Any way: a point that is no pin, or a pin without a direction.
    Free,
    /// Towards the other end: straight at it, or so that the other end is beyond the end of its
    /// lead, `CLEARANCE` its own way.
    Towards,
    /// Across the connection, the other end not beyond the end of its lead: the direction.
    Across((i64, i64)),
    /// Away from the other end.
    Away,
}

/// How a pin at `from` that leaves by `exit` leaves towards `to`.
fn leaving(from: &[Q; 2], exit: Option<(i64, i64)>, to: &[Q; 2]) -> Leaving {
    let Some((x, y)) = exit else { return Leaving::Free };
    let v = [&to[0] - &from[0], &to[1] - &from[1]];
    let along = int(x) * &v[0] + int(y) * &v[1];
    let aside = int(-y) * &v[0] + int(x) * &v[1];
    if along.is_negative() {
        Leaving::Away
    } else if aside.is_zero() || along >= int(CLEARANCE) {
        Leaving::Towards
    } else {
        Leaving::Across((x, y))
    }
}

/// The turns of a straight connection between two points of a net, `a` leaving by `ea` and `b` by
/// `eb`, as the router draws it, a pin leaving by a lead a unit its own way (quality review SC-1):
/// none for a straight line, one for an L, that is a connection not along an axis whose ends
/// leave towards each other; three for a jog, where an end leaves across the connection and the
/// other is not beyond its lead, as the wire must go out, across and back; two for a U, where both
/// leave the same way, across the connection, or off an axis the one beyond the other, as the wire
/// goes out past the farther and comes back to it; four where they leave across opposite ways;
/// and two more for each end that leaves away from the other, as the wire must turn round its
/// symbol.
fn connection_turns(a: &[Q; 2], ea: Option<(i64, i64)>, b: &[Q; 2], eb: Option<(i64, i64)>) -> usize {
    let off_axis = a[0] != b[0] && a[1] != b[1];
    if off_axis && ea.is_some() && ea == eb {
        return 2;
    }
    let (la, lb) = (leaving(a, ea, b), leaving(b, eb, a));
    let away = 2 * usize::from(la == Leaving::Away) + 2 * usize::from(lb == Leaving::Away);
    match (la, lb) {
        (Leaving::Across(x), Leaving::Across(y)) => {
            if x == y {
                2
            } else {
                4
            }
        }
        (Leaving::Across(_), _) | (_, Leaving::Across(_)) => 3,
        _ => usize::from(off_axis) + away,
    }
}

/// The bends of a polyline: vertices where it changes direction.
pub(crate) fn bends(path: &[[Q; 2]]) -> usize {
    let dir = |a: &[Q; 2], b: &[Q; 2]| ((&b[0] - &a[0]).signum(), (&b[1] - &a[1]).signum());
    let mut dirs: Vec<(Q, Q)> =
        path.windows(2).map(|w| dir(&w[0], &w[1])).filter(|d| !(d.0.is_zero() && d.1.is_zero())).collect();
    dirs.dedup();
    dirs.len().saturating_sub(1)
}

/// The pairs of segments of different nets that cross: they share a point inside both, and are
/// not on one line. A horizontal and a vertical segment, which is what rectilinear wires are made
/// of, are compared by their coordinates alone.
fn crossings(segments: &[(usize, [Q; 2], [Q; 2])]) -> Q {
    // (net, the fixed coordinate, the lower and the upper end of the other)
    let mut horizontal: Vec<(usize, &Q, &Q, &Q)> = Vec::new();
    let mut vertical: Vec<(usize, &Q, &Q, &Q)> = Vec::new();
    let mut other: Vec<usize> = Vec::new();
    let span = |a: &'_ Q, b: &'_ Q| if a <= b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
    let mut spans: Vec<(Q, Q)> = Vec::with_capacity(segments.len());
    for (_, a, b) in segments {
        spans.push(if a[1] == b[1] { span(&a[0], &b[0]) } else { span(&a[1], &b[1]) });
    }
    for (i, (net, a, b)) in segments.iter().enumerate() {
        if a == b {
            continue;
        }
        let (lo, hi) = (&spans[i].0, &spans[i].1);
        if a[1] == b[1] {
            horizontal.push((*net, &a[1], lo, hi));
        } else if a[0] == b[0] {
            vertical.push((*net, &a[0], lo, hi));
        } else {
            other.push(i);
        }
    }
    let mut count = 0usize;
    for (nh, y, x1, x2) in &horizontal {
        for (nv, x, y1, y2) in &vertical {
            if nh != nv && *x1 < *x && *x < *x2 && *y1 < *y && *y < *y2 {
                count += 1;
            }
        }
    }
    for (n, &i) in other.iter().enumerate() {
        let (ni, a, b) = &segments[i];
        // against every other segment, the oblique ones once
        for (j, (nj, c, d)) in segments.iter().enumerate() {
            let earlier_oblique = other[..n].contains(&j);
            if j != i && !earlier_oblique && ni != nj && c != d && proper_crossing(a, b, c, d) {
                count += 1;
            }
        }
    }
    count.into_q()
}

/// Whether segments `ab` and `cd` cross at a point inside both, exactly.
fn proper_crossing(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], d: &[Q; 2]) -> bool {
    let cross =
        |o: &[Q; 2], p: &[Q; 2], q: &[Q; 2]| (&p[0] - &o[0]) * (&q[1] - &o[1]) - (&p[1] - &o[1]) * (&q[0] - &o[0]);
    let (d1, d2) = (cross(a, b, c), cross(a, b, d));
    let (d3, d4) = (cross(c, d, a), cross(c, d, b));
    let opposite = |x: &Q, y: &Q| (x.is_positive() && y.is_negative()) || (x.is_negative() && y.is_positive());
    opposite(&d1, &d2) && opposite(&d3, &d4)
}

/// The envelope of the components' boxes: x1, y1, x2, y2; none without a component.
fn envelope(view: &View) -> Option<[Q; 4]> {
    let mut out: Option<[Q; 4]> = None;
    for k in view.s.components() {
        let Some(b) = view.bbox(k) else { continue };
        out = Some(match out {
            None => b,
            Some(e) => [
                e[0].clone().min(b[0].clone()),
                e[1].clone().min(b[1].clone()),
                e[2].clone().max(b[2].clone()),
                e[3].clone().max(b[3].clone()),
            ],
        });
    }
    out
}

/// `|w / h − 16/9|`; for an envelope without height, its width, so that a flat drawing is not
/// taken for a well-proportioned one.
#[cfg(test)]
#[path = "../tests/unit/objectives.rs"]
mod tests;
