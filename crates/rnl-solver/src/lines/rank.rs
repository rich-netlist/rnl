//! Tree-stage ranking for line layout: retained facts, validity rules, then wire quality.

use std::cell::RefCell;

use num_traits::Zero;
use rnl_runtime::problem::{Axis, LevelProblem, PointRef};

use crate::coords::Placed;
use crate::objectives::{Active, ActiveLayer, Objective};
use crate::rational::{int, Q};
use crate::resolve::point_value;
use crate::route::Routing;
use crate::score::Entry;
use crate::structure::Structure;
use crate::SolverOptions;

pub(crate) struct Rank {
    active: Active,
    rules: Vec<usize>,
    annotations: bool,
    seen: RefCell<(Vec<(Q, Q)>, usize)>,
}

impl Rank {
    pub(crate) fn new(s: &Structure) -> Rank {
        let valid = [Objective::Overlaps, Objective::Crowding, Objective::Misleading, Objective::WireOverSymbol];
        let rules: Vec<usize> = (0..s.active.layers.len())
            .filter(|&i| s.active.layers[i].items.iter().any(|(o, _)| valid.contains(o)))
            .collect();
        let annotations = !s.annotation_views.is_empty();
        let n = 2 + 2 * usize::from(annotations) + rules.len() + super::quality::LAB_KEY;
        let layers = (0..n).map(|_| ActiveLayer { items: Vec::new(), tolerance: Q::zero() }).collect();
        Rank {
            active: Active { layers, inert: Vec::new(), configured_only: true },
            rules,
            annotations,
            seen: RefCell::new((Vec::new(), 0)),
        }
    }

    pub(crate) fn active(&self) -> &Active {
        &self.active
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn entry(
        &self,
        problem: &LevelProblem,
        s: &Structure,
        options: &SolverOptions,
        placed: &Placed,
        routing: &Routing,
        objectives: Vec<Q>,
        code: &[u32],
    ) -> Entry {
        let g = &placed.geometry;
        let tol = crate::rational::from_f64(options.tolerance).unwrap_or_else(Q::zero);
        let at = |m: &PointRef, axis: Axis| point_value(problem, g, m, usize::from(axis == Axis::Y), 0);
        let ori = |k: usize| g.orient.get(k).copied();
        let broken = s
            .kept
            .iter()
            .filter(|&&f| crate::holds::check_problem(problem, f, &at, &ori, &tol, &tol) != Some(true))
            .count();
        let mut layers: Vec<Q> = vec![int(broken as i64)];
        if self.annotations {
            let (overlaps, hits) =
                super::quality::annotation_cost(problem, s, placed, routing).expect("validated annotation geometry");
            layers.extend([int(overlaps as i64), int(hits as i64)]);
        }
        layers.push(int(routing.joins_left() as i64));
        layers.extend(self.rules.iter().map(|&i| objectives[i].clone()));
        {
            let mut seen = self.seen.borrow_mut();
            if seen.0.is_empty() {
                seen.0 = layers.iter().map(|v| (v.clone(), v.clone())).collect();
            }
            for (r, v) in seen.0.iter_mut().zip(&layers) {
                if *v < r.0 {
                    r.0 = v.clone();
                }
                if *v > r.1 {
                    r.1 = v.clone();
                }
            }
            seen.1 += 1;
        }
        layers.extend(super::quality::lab_key(problem, s, placed, routing));
        Entry {
            written_path_error: routing.written_path_error,
            annotation_error: routing.annotation_error,
            overlaps: 0,
            crowding: 0,
            misleading: 0,
            joins_left: 0,
            layers,
            code: code.to_vec(),
        }
    }

    pub(crate) fn note(&self, s: &Structure) -> String {
        let seen = self.seen.borrow();
        let mut leading = vec!["facts broken".to_string()];
        if self.annotations {
            leading.extend(["annotation overlaps".into(), "wires through annotations".into()]);
        }
        leading.push("joins left".into());
        let names = leading.into_iter().chain(
            self.rules
                .iter()
                .map(|&i| s.active.layers[i].items.iter().map(|(o, _)| o.name()).collect::<Vec<_>>().join("+")),
        );
        let said: Vec<String> = names
            .zip(&seen.0)
            .map(|(name, (lo, hi))| format!("{name} {}–{}", crate::rational::fmt(lo), crate::rational::fmt(hi)))
            .collect();
        format!(
            "tree: inside the lines layout, {} ranked by the solver's own rules, then crossings before bends: {}",
            crate::driver::count(seen.1, "arrangement"),
            said.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::fact;
    use rnl_runtime::geom::Orient;
    use rnl_runtime::problem::FactKind;

    #[test]
    fn tree_ranking_uses_full_retained_orientation_lists() {
        let (mut p, mut s, placed, routing) = super::super::port::tests::fixture();
        let chosen = placed.geometry.orient[0];
        let other = Orient::ALL.into_iter().find(|o| *o != chosen).unwrap();
        p.facts = vec![fact(FactKind::Orient { object: 0, code: other })];
        p.orient_lists = vec![(0, vec![other, chosen])];
        s.kept = vec![0];
        let rank = Rank::new(&s);
        let layers = vec![int(0); s.active.layers.len()];
        let options = SolverOptions::without_time_limit();
        let good = rank.entry(&p, &s, &options, &placed, &routing, layers.clone(), &[]);
        assert_eq!(good.layers[0], int(0));
        p.orient_lists[0].1.pop();
        let bad = rank.entry(&p, &s, &options, &placed, &routing, layers, &[]);
        assert_eq!(bad.layers[0], int(1));
        assert_eq!(crate::score::order(rank.active(), &[bad, good]), [1, 0]);
    }

    #[test]
    fn tree_results_restore_normal_objectives_after_internal_line_ranking() {
        let (p, s, _, _) = super::super::port::tests::fixture();
        let options = SolverOptions::without_time_limit();
        let normal = crate::tree::Tree::new(&p, &s, &options);
        let ranked = crate::tree::Tree::new(&p, &s, &options).with_line_rank();
        let arr = normal.first(&mut crate::Work::default());
        let (expected, _) = normal.evaluate_rounds(&arr, 0, &mut crate::Work::default());
        let (internal, _) = ranked.evaluate_rounds(&arr, 0, &mut crate::Work::default());
        let actual = ranked.given(internal);
        assert_eq!(actual.placed, expected.placed);
        assert_eq!(actual.routing, expected.routing);
        assert_eq!(actual.entry, expected.entry);
    }
}
