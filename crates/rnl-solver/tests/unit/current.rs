use super::*;
use crate::objectives::{score_term, View};
use crate::rational::{int, Q};
use crate::resolve::tests::{instance, problem_of};
use crate::structure::{analyse, Structure};
use num_traits::Zero;
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{CurrentDirection, CurrentTerminal, PointRef, ProblemCurrent};

fn current(instance: usize, object: usize, pin: usize, net: &str, direction: CurrentDirection) -> ProblemCurrent {
    ProblemCurrent {
        terminal: CurrentTerminal::Instance { instance, terminal: format!("t{pin}") },
        net: net.into(),
        direction,
        display: Some(PointRef::Pin { object, pin, terminal: format!("t{pin}") }),
    }
}

fn structure(problem: &LevelProblem, options: &SolverOptions) -> Structure {
    let resolved = crate::resolve(problem, options);
    analyse(problem, &resolved, options, crate::options::CAPS, &mut Work::default())
}

fn direction(direction: Direction) -> Term {
    Term { direction: Some(direction), ..Term::new("current_direction") }
}

fn problems_of(text: &str) -> Vec<LevelProblem> {
    let files = rnl_runtime::source::MemoryProvider::new().with("t.sp", text);
    rnl_view::interpret("t.sp", None, &files, &rnl_runtime::Options::solver()).unwrap().problems
}

#[test]
fn current_counts_semantic_terminals_independently_on_one_physical_bus_pin() {
    let mut p = problem_of(vec![instance()], Vec::new());
    p.currents = vec![current(0, 0, 1, "a", CurrentDirection::In), current(1, 0, 1, "b", CurrentDirection::Out)];
    let s = structure(&p, &SolverOptions::without_time_limit());
    let at = vec![[int(0), int(0)]];
    let orientations = vec![Orient::R0];
    let v = View { s: &s, at: &at, orient: &orientations };
    assert_eq!(score_term(&direction(Direction::Down), &v, None), int(1));
    assert_eq!(score_term(&direction(Direction::Up), &v, None), int(1));
    p.currents.pop();
    let s = structure(&p, &SolverOptions::without_time_limit());
    assert_eq!(score_term(&direction(Direction::Down), &View { s: &s, ..v }, None), int(0));
    let turned = vec![Orient::R180];
    assert_eq!(score_term(&direction(Direction::Down), &View { s: &s, at: &at, orient: &turned }, None), int(1));
}

#[test]
fn only_explicit_drawn_pin_directions_supply_a_current_score() {
    let mut p = problem_of(vec![instance()], Vec::new());
    p.currents.push(current(0, 0, 1, "a", CurrentDirection::Out));
    let options = SolverOptions::without_time_limit();
    for mode in 0..3 {
        p.symbols[0].pins[1].direction_written = mode == 2;
        p.symbols[0].default = mode == 1;
        let s = structure(&p, &options);
        let at = vec![[Q::zero(), Q::zero()]];
        let ori = vec![Orient::R0];
        let value = score_term(&direction(Direction::Down), &View { s: &s, at: &at, orient: &ori }, None);
        assert_eq!(value, int(i64::from(mode == 2)));
    }
}

#[test]
fn current_graph_compresses_feedback_before_layering_and_ignores_unmarked_bypasses() {
    let mut p = problem_of((0..4).map(|_| instance()).collect(), Vec::new());
    p.currents = vec![
        current(10, 0, 1, "a", CurrentDirection::Out),
        current(20, 1, 1, "a", CurrentDirection::In),
        current(20, 1, 2, "b", CurrentDirection::Out),
        current(30, 2, 1, "b", CurrentDirection::In),
        current(30, 2, 2, "c", CurrentDirection::Out),
        current(20, 1, 0, "c", CurrentDirection::In),
    ];
    let s = structure(&p, &SolverOptions::without_time_limit());
    assert_eq!(s.depth, Some(vec![Some(0), Some(1), Some(1), None]));
    p.currents.push(ProblemCurrent {
        terminal: CurrentTerminal::Port { terminal: "in".into() },
        net: "a".into(),
        direction: CurrentDirection::In,
        display: None,
    });
    let ports = structure(&p, &SolverOptions::without_time_limit());
    assert_eq!(s.depth, ports.depth);
}

#[test]
fn all_configured_current_directions_propose_their_own_deterministic_frames() {
    let mut p = problem_of(vec![instance(), instance()], Vec::new());
    p.currents = vec![current(0, 0, 1, "a", CurrentDirection::Out), current(1, 1, 1, "a", CurrentDirection::In)];
    let mut options = SolverOptions::without_time_limit();
    options.objectives = Objectives {
        layers: vec![Layer {
            items: vec![direction(Direction::Right), direction(Direction::Up), direction(Direction::Left)],
            tolerance: Q::zero(),
        }],
    };
    let s = structure(&p, &options);
    let candidates = crate::search::search(&p, &s, &options, &mut Work::default());
    let frames: std::collections::BTreeSet<_> = candidates.iter().filter_map(|c| c.flow).collect();
    assert_eq!(frames, [Direction::Up, Direction::Left, Direction::Right].into_iter().collect());
}

#[test]
fn symbol_direction_requires_a_reference_when_one_pin_gives_no_cardinal_vector() {
    let mut p = problem_of(vec![instance()], Vec::new());
    p.symbols[0].name = "generic".into();
    p.symbols[0].bbox = [-1.0, -1.0, 1.0, 1.0];
    p.symbols[0].pins.truncate(1);
    p.symbols[0].pins[0].pos = (-1.0, -1.0);
    let s = structure(&p, &SolverOptions::without_time_limit());
    let at = vec![[int(0), int(0)]];
    let ori = vec![Orient::R0];
    let v = View { s: &s, at: &at, orient: &ori };
    let mut t =
        Term { symbols: vec!["generic".into()], direction: Some(Direction::Down), ..Term::new("symbol_direction") };
    assert_eq!(score_term(&t, &v, None), int(0));
    t.reference = Some(Direction::Right);
    assert_eq!(score_term(&t, &v, None), int(1));
    let turn = vec![Orient::R270];
    assert_eq!(score_term(&t, &View { orient: &turn, ..v }, None), int(0));
}

#[test]
fn current_frames_place_vertical_flow_in_rows_and_horizontal_flow_in_columns() {
    let mut p = problem_of(vec![instance(), instance()], Vec::new());
    p.currents = vec![current(0, 0, 1, "a", CurrentDirection::Out), current(1, 1, 1, "a", CurrentDirection::In)];
    for d in [Direction::Down, Direction::Up, Direction::Left, Direction::Right] {
        let mut options = SolverOptions::without_time_limit();
        options.objectives = Objectives { layers: vec![Layer { items: vec![direction(d)], tolerance: Q::zero() }] };
        let (_, exact) = crate::driver::solve_exact(&p, &options);
        let at = &exact.placed.origins;
        let (axis, increasing) = match d {
            Direction::Down => (1, false),
            Direction::Up => (1, true),
            Direction::Left => (0, false),
            Direction::Right => (0, true),
        };
        assert!(if increasing { at[0][axis] < at[1][axis] } else { at[0][axis] > at[1][axis] }, "{d:?}: {at:?}");
    }
}

#[test]
fn disabling_symbol_separation_removes_candidate_extent_gaps_and_lp_repairs() {
    let p = problem_of(vec![instance(), instance()], Vec::new());
    let mut options = SolverOptions::without_time_limit();
    options.objectives = Objectives::none();
    options.separate_symbols = false;
    let s = structure(&p, &options);
    let candidates = crate::search::search(&p, &s, &options, &mut Work::default());
    for c in &candidates {
        let ori = c.orientations(&s);
        for t in crate::search::relations(&s, c, &ori, &options) {
            if matches!(
                t.kind,
                crate::search::RelKind::Gap | crate::search::RelKind::RowsApart | crate::search::RelKind::BlocksApart
            ) {
                match t.rel {
                    crate::search::Relation::Linear { lo, hi, .. } => {
                        assert_eq!(lo, Some(Q::zero()));
                        assert_eq!(hi, None);
                    }
                    crate::search::Relation::Apart { low, high, gap, .. } => {
                        assert!(gap.is_zero());
                        assert!(low.iter().chain(&high).all(|(_, v)| v.is_zero()));
                    }
                    _ => panic!("unexpected separation relation"),
                }
            }
        }
    }
    let (_, exact) = crate::driver::solve_exact(&p, &options);
    assert!(!exact.placed.overlaps.is_empty(), "disabling the mechanism permits overlapping symbols");
}

#[test]
fn attached_symbols_rank_their_orientation_by_the_complete_configuration() {
    let ps = problems_of(include_str!("../data/attached-direction.sp"));
    let p = &ps[0];
    let mut options = SolverOptions::from_toml(
        "[[level]]\n[[level.term]]\nkind='symbol_direction'\nsymbols=['$TAG_SYMBOL']\ndirection='down'",
    )
    .unwrap();
    options.time_limit = None;
    let (solved, exact) = crate::driver::solve_exact(p, &options);
    let tag = p.objects.iter().position(|o| o.label == "%tag").unwrap();
    assert_eq!(exact.placed.geometry.orient[tag], Orient::R0);
    let term = &solved.report.scoring.levels[0].terms[0];
    assert_eq!(term.raw, "0");
    assert!(!term.inactive);
    assert!(exact.structure.class_of(tag).is_some());
}

#[test]
fn virtual_terminals_keep_distinct_physical_pin_networks_and_short_penalties() {
    use crate::objectives::Routed;
    use crate::routing_geometry::{
        GeometryPointId, GeometrySource, GeometryVertex, GeometryWire, OutputWireId, RoutingGeometry,
    };
    let ps = problems_of(include_str!("../data/multi-terminal-virtual.sp"));
    let p = &ps[0];
    let k = p.objects.iter().position(|o| o.label == "%pair").unwrap();
    let s = structure(p, &SolverOptions::without_time_limit());
    for (pin, net) in [(0, "na"), (1, "nb")] {
        let names: Vec<_> =
            s.connections.nets_of[&(k, pin)].iter().map(|&i| s.connections.nets[i].name.as_str()).collect();
        assert_eq!(names, [net]);
    }
    let mut at = vec![[int(100), int(100)]; p.objects.len()];
    at[k] = [int(0), int(0)];
    let orient = vec![Orient::R0; p.objects.len()];
    let id = OutputWireId::Generated(0);
    let vertices = [-1, 1]
        .into_iter()
        .enumerate()
        .map(|(index, y)| GeometryVertex {
            at: [int(-1), int(y)],
            point: GeometryPointId::Bend { wire: id, index },
            physical_pin: None,
        })
        .collect();
    let routed = Routed {
        paths: Vec::new(),
        geometry: RoutingGeometry {
            wires: vec![GeometryWire {
                id,
                vertices,
                nets: ["nb".into()].into_iter().collect(),
                source: GeometrySource::Generated,
            }],
        },
    };
    assert_eq!(crate::score::wire_penalty(&View { s: &s, at: &at, orient: &orient }, Some(&routed), true), 1);
}

#[path = "record_coverage.rs"]
mod record_coverage;

#[test]
fn fixed_record_coverage_rejects_unrecorded_free_axes_and_orientations() {
    let p = problem_of(vec![instance()], Vec::new());
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve(&p, &options);
    let s = structure(&p, &options);
    let mut previous = rnl_runtime::previous::Previous {
        hash: None,
        same_input: None,
        numbering: None,
        placements: Vec::new(),
        wire_routes: Vec::new(),
        wires: Vec::new(),
    };
    let empty = record_coverage::coverage(&p, &resolved, &s, &previous);
    assert_eq!(empty["complete"], false);
    assert_eq!(empty["missing"].as_array().unwrap().len(), 3);
    previous.placements.push(rnl_runtime::solution::Placement {
        object: 0,
        x: Some(7.0),
        y: Some(8.0),
        orient: Some(Orient::R90),
    });
    assert_eq!(record_coverage::coverage(&p, &resolved, &s, &previous)["complete"], true);
    previous.placements[0].y = None;
    let partial = record_coverage::coverage(&p, &resolved, &s, &previous);
    assert_eq!(partial["complete"], false);
    assert_eq!(partial["missing"].as_array().unwrap().len(), 1);
    assert_eq!(partial["missing"][0]["axis"], "y");
}

/// Acceptance-only batch evaluator. Reads completed solver records and measures their existing
/// geometry with v2 definitions; it never searches candidates, places coordinates, or routes.
/// RNL_SCORE_REQUESTS names a JSON array of {"input":"solved.sp","output":"score.json"}.
/// RNL_SCORE_CONFIG optionally names the effective TOML; time is disabled in this evaluation.
#[test]
#[ignore = "requires explicit fixed-record acceptance input/output manifest"]
fn acceptance_score_existing_records() {
    #[derive(serde::Deserialize)]
    struct Request {
        input: String,
        output: String,
    }
    struct Files;
    impl rnl_runtime::source::Provider for Files {
        fn read(&self, path: &str) -> Option<String> {
            std::fs::read_to_string(path).ok()
        }
    }
    use crate::rational::from_f64;
    use crate::routing_geometry::{
        GeometryPointId, GeometrySource, GeometryVertex, GeometryWire, OutputWireId, RoutingGeometry,
    };
    let manifest = std::env::var("RNL_SCORE_REQUESTS").expect("set RNL_SCORE_REQUESTS");
    let requests: Vec<Request> = serde_json::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();
    let mut options = if let Ok(path) = std::env::var("RNL_SCORE_CONFIG") {
        SolverOptions::from_toml(&std::fs::read_to_string(path).unwrap()).unwrap()
    } else {
        SolverOptions::without_time_limit()
    };
    options.time_limit = None;
    for request in requests {
        if let Some(parent) = std::path::Path::new(&request.output).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let without = rnl_view::interpret(&request.input, None, &Files, &rnl_runtime::Options::solver()).unwrap();
        let with = rnl_view::interpret(
            &request.input,
            None,
            &Files,
            &rnl_runtime::Options { problems: true, ..Default::default() },
        )
        .unwrap();
        let errors: Vec<_> = without
            .diagnostics()
            .into_iter()
            .chain(with.diagnostics())
            .filter(|d| d.severity == rnl_runtime::diag::Severity::Error)
            .map(|d| format!("{d:?}"))
            .collect();
        if !errors.is_empty() {
            std::fs::write(
                &request.output,
                serde_json::to_vec_pretty(
                    &serde_json::json!({"score_status":"N/A","reason":"interpretation errors","errors":errors}),
                )
                .unwrap(),
            )
            .unwrap();
            continue;
        }
        let mut levels = Vec::new();
        for p in &without.problems {
            if p.objects.is_empty() {
                continue;
            }
            let previous = rnl_runtime::previous::previous(p, &with)
                .expect("input must have a solver record for every nonempty level");
            assert_ne!(previous.same_input, Some(false), "stale input: {} / {}", request.input, p.title);
            assert_ne!(previous.numbering, Some(false), "stale numbering: {} / {}", request.input, p.title);
            // Resolution supplies only authored fixed/relative coordinates and fact membership.
            // Every recorded axis/orientation is copied over it, without coordinate optimization.
            let resolved = crate::resolve(p, &options);
            let s = analyse(p, &resolved, &options, crate::options::CAPS, &mut Work::default());
            let coverage = record_coverage::coverage(p, &resolved, &s, &previous);
            if coverage["complete"] != true {
                levels.push(serde_json::json!({"level":p.level,"title":p.title,"score_status":"N/A","reason":"incomplete fixed-record geometry","coverage":coverage}));
                continue;
            }
            let mut g = resolved.witness.geometry.clone();
            for placement in &previous.placements {
                if let Some(x) = placement.x {
                    g.origin[placement.object][0] = from_f64(x).unwrap();
                }
                if let Some(y) = placement.y {
                    g.origin[placement.object][1] = from_f64(y).unwrap();
                }
                if let Some(orient) = placement.orient {
                    g.orient[placement.object] = orient;
                }
            }
            let at = s.origins(p, &g);
            let mut geometry = RoutingGeometry {
                wires: (0..p.wires.len())
                    .map(|i| {
                        crate::routing_geometry::authored_wire(p, &g, i).unwrap_or_else(|| {
                            panic!("{} / {}: authored wire {i} cannot be reconstructed", request.input, p.title)
                        })
                    })
                    .collect(),
            };
            let vertices = |points: &[(f64, f64)], id| {
                points
                    .iter()
                    .enumerate()
                    .map(|(index, &(x, y))| GeometryVertex {
                        at: [from_f64(x).unwrap(), from_f64(y).unwrap()],
                        // PreviousWire contains output points rather than endpoint references. Preserve
                        // independent output-vertex identities; do not invent a shared identity by position.
                        point: GeometryPointId::Bend { wire: id, index },
                        physical_pin: None,
                    })
                    .collect()
            };
            for route in &previous.wire_routes {
                let wire = &mut geometry.wires[route.wire];
                wire.vertices = vertices(&route.points, OutputWireId::Authored(route.wire));
            }
            for (i, wire) in previous.wires.iter().enumerate() {
                let id = OutputWireId::Generated(i);
                geometry.wires.push(GeometryWire {
                    id,
                    vertices: vertices(&wire.points, id),
                    nets: wire.nets.iter().cloned().collect(),
                    source: GeometrySource::Generated,
                });
            }
            let routing = crate::route::Routing { geometry, ..Default::default() };
            let drawn = crate::score::drawn(p, &s, &g, &routing);
            let report = s.active.report(&View { s: &s, at: &at, orient: &g.orient }, Some(&drawn), &options);
            levels.push(serde_json::json!({"level":p.level,"title":p.title,"score_status":"complete","coverage":coverage,"scoring":report,"geometry_wires":drawn.geometry.wires.len(),"record_placements":previous.placements.len(),"record_wires":previous.wires.len(),"search_performed":false,"routing_performed":false}));
        }
        let output = std::path::Path::new(&request.output);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(output, serde_json::to_vec_pretty(&levels).unwrap()).unwrap();
    }
}
