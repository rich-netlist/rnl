//! Opt-in evidence from a new production solve, never recovery of an earlier in-memory result.
//! Capture only moves the selected state. Evaluation and JSON allocation belong after all levels.

use rnl_runtime::problem::LevelProblem;
use serde_json::{json, Value};

use crate::driver::Exact;
use crate::objectives::{inert_reason, score, Inputs, Objective, View};
use crate::rational::{from_f64, int, Q};
use crate::structure::PortKind;
use crate::SolverOptions;

/// An opaque, already selected exact result. Keeping it alive increases diagnostic-run memory;
/// no evaluator, extra placement, routing, solving, clock read or formatting runs on capture.
#[derive(Clone, Debug)]
pub struct Captured(pub(crate) Exact);

fn q(value: &Q) -> String {
    format!("{}/{}", value.numer(), value.denom())
}

fn point(value: &[Q; 2]) -> [String; 2] {
    [q(&value[0]), q(&value[1])]
}

impl Captured {
    /// Evaluate the selected state with the original objective evaluator, after the complete
    /// file solve (including any version-raise pass). Inert raw values remain null, never zero.
    /// The full exact context uses Rust Debug text in addition to the structured Q score/layout;
    /// it is evidence, not a deserialization or standalone replay format.
    pub fn evaluate(&self, problem: &LevelProblem, options: &SolverOptions) -> Value {
        let e = &self.0;
        let s = &e.structure;
        let wires = crate::score::drawn(problem, s, &e.placed.geometry, &e.routing);
        let view = View { s, at: &e.placed.origins, orient: &e.placed.geometry.orient };
        // Exactly the production Active::new input predicates, from the retained Structure.
        let inputs = Inputs {
            supply: !s.power.supply.is_empty(),
            ground: !s.power.ground.is_empty(),
            port_kinds: s.ports.iter().any(|p| p.kind != PortKind::Unknown),
            wires: !problem.routes.is_empty() || !problem.wires.is_empty(),
        };
        let mut active_index = 0;
        let configured: Vec<Value> = options
            .objectives
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let items: Vec<Value> = layer
                    .items
                    .iter()
                    .map(|(name, weight)| {
                        let objective = Objective::from_name(name);
                        let reason = objective.and_then(|o| inert_reason(o, &inputs));
                        let raw = objective.filter(|_| reason.is_none()).map(|o| score(o, &view, Some(&wires)));
                        json!({"name": name, "weight": weight, "active": raw.is_some(),
                    "inert_reason": reason, "raw_q": raw.as_ref().map(q),
                    "weighted_q": raw.as_ref().map(|v| q(&(v * int(i64::from(*weight)))))})
                    })
                    .collect();
                let active = items.iter().any(|item| item["active"] == true);
                let mapped = active.then_some(active_index);
                if active {
                    active_index += 1;
                }
                json!({"configured_index": index, "active_index": mapped, "items": items,
                "tolerance_f64": layer.tolerance, "tolerance_q": from_f64(layer.tolerance).as_ref().map(q)})
            })
            .collect();
        let active: Vec<Value> = s
            .active
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                json!({"index": index, "items": layer.items.iter().map(|(o, weight)|
                json!({"name": o.name(), "weight": weight})).collect::<Vec<_>>(),
                "tolerance_q": q(&layer.tolerance), "selected_weighted_q": e.score.get(index).map(q)})
            })
            .collect();
        let objects: Vec<Value> = problem
            .objects
            .iter()
            .enumerate()
            .map(|(index, object)| {
                json!({"index": index, "reference_debug": format!("{:?}", object.reference), "label": object.label,
                "has_own_position": s.objects.get(index).is_some_and(|o| o.own),
                "own_origin_q": e.placed.geometry.origin.get(index).filter(|_| s.objects[index].own).map(point),
                "drawn_origin_q": e.placed.origins.get(index).map(point),
                "orientation": e.placed.geometry.orient.get(index).filter(|_| object.orient.is_some()).map(|o| o.as_str()),
                "left_free": s.objects.get(index).is_some_and(|o| o.left_free)})
            })
            .collect();
        let recomputed = s.active.score(&view, Some(&wires));
        json!({
            "evidence_kind": "new_production_diagnostic_exact",
            "historical_memory_exact": "not_saved_unknown",
            "saved_artifact_binding": "pending_byte_for_byte_comparison_and_metadata_defaults_facts_checks",
            "problem_sig2": rnl_runtime::problem::sig2(problem, &options.canonical()),
            "problem_canonical": rnl_runtime::problem::canonical(problem),
            "problem_debug": format!("{problem:#?}"),
            "options_canonical": options.canonical(), "hints_for_low": options.hints_for_low,
            "selected_stage": "final_routed_candidate", "candidate_id": e.candidate_id,
            "candidate_origin": e.candidate.origin, "candidate_code": e.code,
            "rank_counts_order": ["overlaps", "crowding", "misleading", "joins_left"], "rank_counts": e.rank,
            "configured_layers": configured, "active_layers": active, "inert_reasons": s.active.inert,
            "selected_score_q": e.score.iter().map(q).collect::<Vec<_>>(),
            "recomputed_active_score_q": recomputed.iter().map(q).collect::<Vec<_>>(),
            "selected_score_matches_recomputed": e.score == recomputed,
            "configured_active_layer_count_matches": active_index == s.active.layers.len(),
            "retained_fact_indices": e.resolved.kept, "structure_fact_indices": s.kept,
            "objects": objects,
            "routed_paths": e.routing.paths.iter().enumerate().map(|(index, path)| json!({
                "wire_index": index, "drawn_net_index": e.routing.drawn.get(index),
                "points_q": path.iter().map(point).collect::<Vec<_>>() })).collect::<Vec<_>>(),
            "scored_paths": wires.paths.iter().map(|(net, path)| json!({"drawn_net_index": net,
                "points_q": path.iter().map(point).collect::<Vec<_>>() })).collect::<Vec<_>>(),
            "exact_context_debug": format!("{e:#?}"),
            "context_note": "Debug context retains resolved/structure/candidate/placed/routing and exact Q; not a replay input."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_text_keeps_a_value_that_f64_cannot_distinguish() {
        let value = int(9_007_199_254_740_992) + crate::rational::frac(1, 3);
        assert_eq!(q(&value), "27021597764222977/3");
    }

    #[test]
    fn capture_uses_the_callers_clock_and_inert_raw_items_stay_null() {
        use crate::resolve::tests::{fact, point as problem_point, problem_of, Ticks};
        use rnl_runtime::problem::{Axis, FactKind, PointRef};

        let p = problem_of(
            vec![problem_point()],
            vec![fact(FactKind::Place { at: PointRef::Origin(0), axis: Axis::X, value: 3.0 })],
        );
        let options = SolverOptions::default();
        let plain_clock = Ticks::default();
        let captured_clock = Ticks::default();
        let plain = crate::solve_with_clock(&p, &options, &plain_clock);
        let (solved, captured) = crate::solve_with_clock_quality(&p, &options, &captured_clock);
        assert_eq!(plain, solved);
        assert_eq!(plain_clock.reads(), captured_clock.reads());
        let reads_before_evaluation = captured_clock.reads();
        let evidence = captured.evaluate(&p, &options);
        assert_eq!(captured_clock.reads(), reads_before_evaluation);
        assert_eq!(evidence["configured_layers"].as_array().unwrap().len(), 11);
        assert_eq!(evidence["selected_score_q"], evidence["recomputed_active_score_q"]);
        let current = evidence["configured_layers"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|l| l["items"].as_array().unwrap())
            .find(|i| i["name"] == "current_direction")
            .unwrap();
        assert_eq!(current["active"], false);
        assert!(current["raw_q"].is_null() && current["weighted_q"].is_null());
        assert!(current["inert_reason"].as_str().unwrap().contains("nothing to work with"));
    }
}
