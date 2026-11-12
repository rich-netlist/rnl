use rnl_runtime::{
    diag::Severity,
    problem::{LevelProblem, PointRef, Reference},
    solution::{write_record, Solution},
    source::MemoryProvider,
    Options,
};
use rnl_solver_io::{decode_solution, encode_solution};
use serde_json::json;

fn problem(version: &str) -> LevelProblem {
    let text = format!(
        r#"* protocol round trip
* @RNL(type="header") {{ @VERSION("{version}") }}
R1 a n 1k
R2 n b 1k
* @RNL {{
* $S = @DEFINE-SYMBOL {{ (-1,-1) (1,1) @RECTANGLE; (-2,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) }};
* {{#R1 #R2}} @SET-SYMBOL $S;
* &n @ROUTE;
* &a @WIRE(mode="straight") {{#R1:1 (-6,0)}}
* }}
.end
"#
    );
    let text = if version == "0.4" { text.replace("@ROUTE;", "@ROUTE-NET;") } else { text };
    let files = MemoryProvider::new().with("test.sp", &text);
    let read = rnl_view::interpret("test.sp", None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(read.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", read.diagnostics());
    read.problems[0].clone()
}

#[test]
fn actual_native_results_and_written_records_round_trip_for_each_language_version() {
    for version in ["0.2", "0.3", "0.4"] {
        let p = problem(version);
        let solved = rnl_solver::solve(&p, &rnl_solver::SolverOptions::without_time_limit());
        assert!(!solved.solution.placements.is_empty());
        assert!(!solved.solution.wires.is_empty());
        let json = encode_solution(&solved.solution).unwrap();
        let bytes = serde_json::to_vec(&json).unwrap();
        let decoded = decode_solution(&serde_json::from_slice(&bytes).unwrap(), &p).unwrap();
        assert_eq!(decoded, solved.solution);
        assert_eq!(
            write_record(&p, &decoded, "test", "input").record,
            write_record(&p, &solved.solution, "test", "input").record
        );
    }
}

#[test]
fn pin_coordinates_and_electrical_pins_are_not_collapsed_by_the_protocol() {
    let p = problem("0.4");
    let PointRef::Pin { object, pin, terminal } = &p.wires[0].anchors[0] else {
        panic!("expected an authored terminal");
    };
    let reference = p.objects[*object].reference.then(&format!(":{}", rnl_runtime::problem::written_name(terminal)));
    let s = Solution {
        junctions: vec![
            PointRef::SymbolPin { object: *object, pin: *pin, reference: reference.clone() },
            PointRef::PinPosition { object: *object, pin: *pin, reference: reference.then("::position") },
        ],
        ..Solution::default()
    };
    let decoded = decode_solution(&encode_solution(&s).unwrap(), &p).unwrap();
    assert_eq!(decoded, s);
    assert!(decoded.junctions[0].pin_key().is_some());
    assert!(decoded.junctions[1].pin_key().is_none());
}

#[test]
fn a_valid_pin_index_does_not_authorize_another_endpoint_or_arbitrary_reference_text() {
    let p = problem("0.4");
    for reference in ["#R2:2", "#R1:1", "#R1:1::position; (99,99) @PLACE(0,0)"] {
        let s = Solution {
            junctions: vec![PointRef::PinPosition { object: 0, pin: 0, reference: Reference::Name(reference.into()) }],
            ..Solution::default()
        };
        assert!(decode_solution(&encode_solution(&s).unwrap(), &p).is_err(), "{reference}");
    }
    let other_terminal = p
        .nets
        .iter()
        .flat_map(|n| &n.pins)
        .find_map(|r| match r {
            PointRef::Pin { object: 0, pin: 1, terminal } => Some(terminal.clone()),
            _ => None,
        })
        .unwrap();
    let s = Solution {
        junctions: vec![PointRef::Pin { object: 0, pin: 0, terminal: other_terminal }],
        ..Solution::default()
    };
    assert!(decode_solution(&encode_solution(&s).unwrap(), &p).is_err());
}

#[test]
fn malformed_result_indices_and_duplicate_placements_are_rejected_with_paths() {
    let p = problem("0.4");
    let solved = rnl_solver::solve(&p, &rnl_solver::SolverOptions::without_time_limit());
    let original = encode_solution(&solved.solution).unwrap();
    let mut bad = original.clone();
    bad["placements"][0]["object"] = json!(p.objects.len());
    assert_eq!(decode_solution(&bad, &p).unwrap_err().path, "$.placements[0].object");
    let mut bad = original.clone();
    let duplicate = bad["placements"][0].clone();
    bad["placements"].as_array_mut().unwrap().push(duplicate);
    assert!(decode_solution(&bad, &p).unwrap_err().message.contains("duplicate"));
    let mut bad = original.clone();
    bad["wires"][0]["route"] = json!(p.routes.len());
    assert_eq!(decode_solution(&bad, &p).unwrap_err().path, "$.wires[0].route");
    let mut bad = original.clone();
    bad["wires"][0]["points"][0] = json!({"kind":"branch","branch":solved.solution.branches.len()});
    assert_eq!(decode_solution(&bad, &p).unwrap_err().path, "$.wires[0].points[0].branch");
    let mut bad = original;
    bad["wires"][0]["points"][0] = json!({"kind":"anchor","point":{"kind":"pin","object":0,"pin":999,"terminal":"p"}});
    assert_eq!(decode_solution(&bad, &p).unwrap_err().path, "$.wires[0].points[0].point.pin");
}

#[test]
fn nonfinite_native_coordinates_cannot_turn_into_absent_decisions() {
    let p = problem("0.4");
    let mut solved = rnl_solver::solve(&p, &rnl_solver::SolverOptions::without_time_limit());
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        solved.solution.placements[0].x = Some(value);
        assert_eq!(encode_solution(&solved.solution).unwrap_err().path, "$.placements[0].x");
    }
}

fn result(p: &LevelProblem) -> rnl_solver_io::ResultDocument {
    let options = rnl_solver::SolverOptions::without_time_limit();
    let solved = rnl_solver::solve(p, &options);
    rnl_solver_io::ResultDocument {
        solver: rnl_solver_io::SolverIdentity { name: "fixture".into(), version: "1.0".into() },
        levels: vec![rnl_solver_io::LevelResult {
            request: 0,
            level: p.level,
            input: rnl_runtime::problem::input_digest(p),
            options: solved.report.canonical_options.clone().unwrap_or_else(|| options.canonical()),
            solution: solved.solution,
            report: Some(solved.report),
        }],
    }
}

#[test]
fn complete_native_reports_round_trip_without_losing_replay_settings() {
    for version in ["0.2", "0.3", "0.4"] {
        let p = problem(version);
        let result = result(&p);
        let value = rnl_solver_io::encode_result(&result).unwrap();
        assert_eq!(rnl_solver_io::decode_result(&value, &[p]).unwrap(), result);
    }
}

#[test]
fn result_envelope_rejects_stale_missing_extra_and_duplicate_levels() {
    let p = problem("0.4");
    let result = result(&p);
    let original = rnl_solver_io::encode_result(&result).unwrap();
    for mutation in 0..7 {
        let mut value = original.clone();
        match mutation {
            0 => value["version"] = json!(rnl_solver_io::VERSION + 1),
            1 => value["levels"][0]["input"] = json!("different-input"),
            2 => {
                value["levels"].as_array_mut().unwrap().clear();
            }
            3 => {
                let duplicate = value["levels"][0].clone();
                value["levels"].as_array_mut().unwrap().push(duplicate);
            }
            4 => value["levels"][0]["request"] = json!(1),
            5 => value["levels"][0]["report"]["kept"] = json!([p.facts.len()]),
            _ => value["levels"][0]["options"] = json!("different-settings"),
        }
        assert!(rnl_solver_io::decode_result(&value, std::slice::from_ref(&p)).is_err(), "mutation {mutation}");
    }
}

#[test]
fn request_ordinals_disambiguate_identical_inputs_and_responses_can_be_reordered() {
    let p = problem("0.4");
    let mut result = result(&p);
    let mut second = result.levels[0].clone();
    second.request = 1;
    result.levels.push(second);
    let mut json = rnl_solver_io::encode_result(&result).unwrap();
    json["levels"].as_array_mut().unwrap().reverse();
    assert_eq!(rnl_solver_io::decode_result(&json, &[p.clone(), p]).unwrap(), result);
}

#[test]
fn third_party_result_without_a_fact_report_stays_unreported() {
    let p = problem("0.4");
    let mut result = result(&p);
    result.levels[0].report = None;
    let json = rnl_solver_io::encode_result(&result).unwrap();
    assert!(rnl_solver_io::decode_result(&json, &[p]).unwrap().levels[0].report.is_none());
}

#[test]
fn nonempty_solution_and_report_fields_survive_bytes_including_large_work_counts() {
    use rnl_runtime::solution::{Branch, RoutedWire, WirePoint, WireRoute};
    use rnl_solver::{
        BudgetReport, Discarded, RunLimit, SolveReport, Statement, StatementHint, StopReason, Unrouted, Work,
    };
    let source = "* report\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\",version=\"0.2\") }\nV1 a 0 1\nR1 a 0 1k\n* @RNL { (9,9) @TEXT(\"note\"); &a @ROUTE }\n.end\n";
    let files = MemoryProvider::new().with("report.sp", source);
    let read = rnl_view::interpret("report.sp", None, &files, &Options::solver()).unwrap();
    assert!(read.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", read.diagnostics());
    let p = &read.problems[0];
    assert!(p.facts.len() >= 3 && !p.usages.is_empty() && !p.routes.is_empty() && !p.wires.is_empty());
    assert!(p.wires.iter().any(|w| matches!(w.reference, Reference::GeneratedNumber { .. })));
    assert!(p.objects.iter().any(|o| matches!(o.reference, Reference::Generated { .. })));
    assert!(p.objects.iter().any(|o| matches!(o.reference, Reference::Number { .. })));
    let mut document = result(p);
    let level = &mut document.levels[0];
    level.options = "fixture-options".into();
    level.solution.branches =
        vec![Branch { at: (3.25, -7.5), junction: false }, Branch { at: (4.5, -6.25), junction: true }];
    level.solution.wires = vec![RoutedWire {
        route: 0,
        points: vec![
            WirePoint::Anchor(p.routes[0].pins[0].clone()),
            WirePoint::Branch(0),
            WirePoint::Bend((5.5, -2.25)),
            WirePoint::Branch(1),
            WirePoint::Anchor(p.routes[0].pins[1].clone()),
        ],
    }];
    level.solution.wire_routes =
        vec![WireRoute { wire: 0, points: p.wires[0].anchors.iter().cloned().map(WirePoint::Anchor).collect() }];
    let report = SolveReport {
        canonical_options: Some(level.options.clone()),
        kept: vec![0],
        failed: vec![2],
        discarded: vec![Discarded { fact: 1, groups: vec![vec![0]], complete: false, undecided: false }],
        statements: std::collections::BTreeMap::from([(
            Statement { file: "report.sp".into(), line: 5, col: 8, chain: vec![0], point: Some(0) },
            StatementHint { facts: vec![0], discarded: vec![1] },
        )]),
        inert_objectives: vec!["fixture objective".into()],
        unrouted: vec![Unrouted { route: 0, net: p.routes[0].net.clone(), parts: 2, budget: true }],
        work: Work { resolve_nodes: 9007199254740993, pivots: 17, candidates: 23, astar_expansions: 29 },
        errors: vec!["fixture error".into()],
        warnings: vec!["fixture warning".into()],
        notes: vec!["α \"β\"".into()],
        ..SolveReport::default()
    };
    for (limit, reason) in [
        (RunLimit::Time(0.125), StopReason::Time),
        (RunLimit::Work(9007199254740993), StopReason::Work),
        (RunLimit::Seed, StopReason::Complete),
        (RunLimit::Unlimited, StopReason::Safety),
    ] {
        let mut report = report.clone();
        report.budget = Some(BudgetReport { limit, reason, work: 9007199254740993 });
        document.levels[0].report = Some(report);
        let bytes = serde_json::to_vec(&rnl_solver_io::encode_result(&document).unwrap()).unwrap();
        let decoded =
            rnl_solver_io::decode_result(&serde_json::from_slice(&bytes).unwrap(), std::slice::from_ref(p)).unwrap();
        assert_eq!(decoded, document);
    }
}
