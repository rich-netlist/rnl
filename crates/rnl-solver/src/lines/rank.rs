//! Tree-stage ranking for line layout: retained facts that must hold, false joins, the facts of
//! low priority, validity rules, then wire quality.

use std::cell::RefCell;

use num_traits::Zero;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::problem::{Axis, LevelProblem, PointRef};

use crate::coords::Placed;
use crate::objectives::{Active, ActiveLayer};
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
    /// Below a `virtual_weight` of 1, the weight: the heights and the rows and columns of the
    /// symbols weighed, each layer with a tolerance just under a whole unit ([`Rank::for_choice`]).
    weighed: Option<Q>,
    seen: RefCell<(Vec<(Q, Q)>, usize)>,
}

/// The layers of the objectives in force that hold the lines layout's rules.
pub(super) fn rule_layers(s: &Structure) -> Vec<usize> {
    let valid = crate::config::LINES_OBJECTIVES;
    (0..s.active.layers.len()).filter(|&i| s.active.layers[i].items.iter().any(|(o, _)| valid.contains(o))).collect()
}

impl Rank {
    pub(crate) fn new(s: &Structure) -> Rank {
        let rules = rule_layers(s);
        let annotations = !s.annotation_views.is_empty();
        let n = 4 + 2 * usize::from(annotations) + rules.len() + 6 + 4;
        let layers = (0..n).map(|_| ActiveLayer { items: Vec::new(), tolerance: Q::zero() }).collect();
        Rank {
            active: Active { layers, inert: Vec::new(), configured_only: true },
            rules,
            annotations,
            weighed: None,
            seen: RefCell::new((Vec::new(), 0)),
        }
    }

    /// The rank by which the line layout chooses between its drawing and the group-tree layout's:
    /// [`Rank::new`], and below a `virtual_weight` of 1, the heights of the grounds and supplies and
    /// the rows and columns of the symbols weighed by it, two drawings alike on each unless they
    /// differ by a whole unit. The group-tree layout's own search ranks by [`Rank::new`].
    pub(crate) fn for_choice(s: &Structure, options: &SolverOptions) -> Rank {
        let mut rank = Rank::new(s);
        let w = options.settings.virtual_weight;
        if !w.is_one() {
            let base = 4 + 2 * usize::from(rank.annotations) + rank.rules.len();
            for i in [base + 3, base + 6 + 2] {
                rank.active.layers[i].tolerance = super::quality::unit_tolerance();
            }
            rank.weighed = Some(w.q());
        }
        rank
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
        let broken: Vec<usize> = s
            .kept
            .iter()
            .copied()
            .filter(|&f| crate::holds::check_problem(problem, f, &at, &ori, &tol, &tol) != Some(true))
            .collect();
        let low = broken.iter().filter(|&&f| problem.facts[f].priority == Priority::Low).count();
        // a drawing that reads as connected where it is not is wrong: it ranks after the facts that
        // must hold and before those of low priority, which a fill writes as a preference
        let joins = crate::score::drawn(problem, s, g, routing)
            .map_or(0, |wires| crate::score::false_joins(problem, s, &placed.origins, g, &wires));
        let mut layers: Vec<Q> = vec![int((broken.len() - low) as i64), int(joins as i64), int(low as i64)];
        layers.push(int(routing.joins_left() as i64));
        if self.annotations {
            let (overlaps, hits) =
                super::quality::annotation_cost(problem, s, placed, routing).expect("validated annotation geometry");
            layers.extend([int(overlaps as i64), int(hits as i64)]);
        }
        // the unified key: the line-layout order, its crossings and bends weighed by the one pair,
        // with the configured rules between the annotations and its marks
        let w = self.weighed.clone().unwrap_or_else(|| int(1));
        let l =
            super::quality::lab_measure(problem, s, placed, routing, options.settings.lines_weights.pair(), &w, None);
        if !l.interrupted {
            layers.extend(self.rules.iter().map(|&i| objectives.get(i).cloned().unwrap_or_else(Q::zero)));
        }
        if !l.interrupted {
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
        if l.interrupted {
            layers.extend((0..self.rules.len() + 6 + 4).map(|_| int(i64::MAX)));
        } else if self.weighed.is_some() {
            layers.extend(l.head_weighed());
            layers.extend(l.tail_weighed());
        } else {
            layers.extend(l.head());
            layers.extend(l.tail());
        }
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
        let mut leading =
            vec!["facts broken".to_string(), "false joins".into(), "low facts broken".into(), "joins left".into()];
        if self.annotations {
            leading.extend(["annotation overlaps".into(), "wires through annotations".into()]);
        }
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
            "tree: inside the lines layout, {} ranked by the solver's own rules, then the line-layout order: {}",
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

    /// At a `virtual_weight` of 1 the rank of the line layout's choice is the tree's: no
    /// tolerance, the same layers. Below 1 the heights and the rows and columns of the symbols
    /// take a tolerance just under a whole unit.
    #[test]
    fn the_choice_ranks_as_the_tree_at_a_weight_of_one() {
        let (p, s, placed, routing) = super::super::port::tests::fixture();
        let options = SolverOptions::without_time_limit();
        let layers = vec![int(0); s.active.layers.len()];
        let (tree, choice) = (Rank::new(&s), Rank::for_choice(&s, &options));
        assert!(choice.weighed.is_none());
        assert!(choice.active().layers.iter().all(|l| l.tolerance.is_zero()));
        let a = tree.entry(&p, &s, &options, &placed, &routing, layers.clone(), &[]);
        let b = choice.entry(&p, &s, &options, &placed, &routing, layers.clone(), &[]);
        assert_eq!(a.layers, b.layers);
        let half = options.with_overrides("virtual_weight=0.5").unwrap();
        let weighed = Rank::for_choice(&s, &half);
        let base = 4 + 2 * usize::from(weighed.annotations) + weighed.rules.len();
        let tolerant: Vec<usize> =
            (0..weighed.active().layers.len()).filter(|&i| !weighed.active().layers[i].tolerance.is_zero()).collect();
        assert_eq!(tolerant, [base + 3, base + 8]);
        assert!(weighed.active().layers[base + 3].tolerance < int(1));
        // the fixture's port is a virtual symbol on a column of its own, which counts one half; the
        // other layers are the counts
        let c = weighed.entry(&p, &s, &half, &placed, &routing, layers, &[]);
        let mut expected = a.layers.clone();
        expected[base + 8] = &expected[base + 8] - int(1) / int(2);
        assert_eq!(c.layers, expected);
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
