//! Real-interpreter checks for junction completion; no test doubles for RNL interpretation.
use rnl_cli::{
    interpret_file,
    junction_completion_driver::{complete_record, Context},
};
use rnl_runtime::{
    solution::{write_record, Branch, Solution},
    source::MemoryProvider,
    Options,
};
use std::{collections::BTreeMap, path::Path};

// A backend may attach to an existing named point or emit a new branch point.
// Completion must preserve the requested coordinates and fixed axes in either case.
fn completion_suffix<'a>(record: &'a str, definition: &str) -> &'a str {
    let prefix = format!("{definition} @ATTACH ");
    let mut statements = record.lines().filter_map(|line| line.split_once(&prefix).map(|(_, rest)| rest));
    let statement = statements.next().expect("one configured junction usage");
    assert!(statements.next().is_none(), "unexpected duplicate junction usage: {record}");
    statement.split_once(" => ").map_or("", |(_, suffix)| suffix.trim().trim_end_matches(';'))
}

fn run(body: &str) -> Result<String, String> {
    let text = format!("* junction\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }}\nR1 a 0 1k\n* @RNL {{ {body} }}\n.end\n");
    let files = MemoryProvider::new().with("main.sp", &text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let interpreted = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &interpreted.problems[0];
    let written = write_record(
        p,
        &Solution { branches: vec![Branch { at: (0., 0.), junction: true }], ..Default::default() },
        "test",
        "test",
    );
    let formats = BTreeMap::new();
    let ctx = Context {
        entry: "main.sp",
        dialect: None,
        formats: &formats,
        files: &files,
        runtime: &opts,
        original: &interpreted,
        problem: p,
        original_text: &text,
        host_path: "main.sp",
        kept: &[],
        solution: &Solution::default(),
        tolerance: opts.tolerance,
        on_interpret: None,
    };
    complete_record(&ctx, &written).map(|c| c.record)
}

#[test]
fn real_interpreter_aligns_standard_shape_dot() {
    let result =
        run(r#"$DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") }; %p=(3,4)"#).unwrap();
    assert!(result.contains("$DOT @ATTACH %rt1 => @PLACE(0, 0)"), "{result}");
}

#[test]
fn real_interpreter_uses_offset_not_role_point() {
    let result = run(r#"$DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right", exact=2); {? ?p} @ALIGN("y") }; %p=(3,4)"#).unwrap();
    assert!(result.contains("$DOT @ATTACH %rt1 => @PLACE(2, 0)"), "{result}");
}

#[test]
fn inconsistent_definition_cannot_emit_replacement() {
    let result = run(
        r#"$DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right", exact=2); ?p ? @DISTANCE("left-to-right", exact=3) }; %p=(3,4)"#,
    );
    assert!(result.is_err(), "{result:?}");
}

#[test]
fn real_interpreter_official_standard_junction() {
    let text = "* official dot\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-basic\", version=\"0.4\"); @SOLVER-CONFIG(junction=\"$JUNCTION\") }\nR1 a 0 1k\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let interpreted = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &interpreted.problems[0];
    let written = write_record(
        p,
        &Solution { branches: vec![Branch { at: (0., 0.), junction: true }], ..Default::default() },
        "test",
        "test",
    );
    let formats = BTreeMap::new();
    let ctx = Context {
        entry: "main.sp",
        dialect: None,
        formats: &formats,
        files: &files,
        runtime: &opts,
        original: &interpreted,
        problem: p,
        original_text: text,
        host_path: "main.sp",
        kept: &[],
        solution: &Solution::default(),
        tolerance: opts.tolerance,
        on_interpret: None,
    };
    let result = complete_record(&ctx, &written).unwrap().record;
    assert!(result.contains("$JUNCTION @ATTACH %rt1 => @PLACE(0, 0)"), "{result}");
}

#[test]
fn backend_runs_once_and_a_dot_that_cannot_be_placed_is_left_out() {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    use std::cell::Cell;
    let text = r#"* transaction
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a 0 1k
* @RNL { $DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right",exact=2); ?p ? @DISTANCE("left-to-right",exact=3) } }
* @RNL(type="generated",priority="high",author="old",hash="old") { %old=(5,5) }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let calls = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|_, _| {
            calls.set(calls.get() + 1);
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        branches: vec![Branch { at: (0., 0.), junction: true }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                author: "test".into(),
                report_available: true,
            })
        },
    );
    // the dot costs only itself: the record of the level replaces the old one, without it
    let result = result.expect("the level is written without the dot");
    assert_eq!(calls.get(), 1);
    let record = &result.levels[0].record;
    assert!(!record.contains("$DOT @ATTACH") && record.contains("%rt1 = (0, 0)"), "{record}");
    let unsolved = &result.levels[0].written.unsolved;
    assert_eq!(unsolved.len(), 1, "{unsolved:?}");
    assert_eq!(unsolved[0].code, rnl_runtime::diag::codes::UNSOLVED_JUNCTION_UNPLACED);
    let written = &result.files.iter().find(|file| file.path == "main.sp").expect("written main").text;
    assert!(!written.contains("%old=(5,5)"), "{written}");
}

#[test]
fn actual_solve_routes_and_locates_new_dot() {
    use rnl_cli::solve::{solve, SolveOptions};
    let text = include_str!("junction_completion_fixtures/no-existing-dot.sp");
    let files = MemoryProvider::new().with("main.sp", text);
    let options = SolveOptions { std: false, ..Default::default() };
    let result = solve(Path::new("main.sp"), &files, &options).expect("solve and junction completion");
    let file = result.files.iter().find(|file| file.path == "main.sp").expect("written main");
    assert_eq!(completion_suffix(&file.text, "$DOT"), "@PLACE(0, 0)", "{}", file.text);
    let saved = MemoryProvider::new().with("main.sp", &file.text);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &saved,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(read.model.levels[0].unplaced.is_empty(), "{:?}", read.model.levels[0].unplaced);
    assert!(!read.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::NO_DOT));
}

#[test]
fn actual_solve_preserves_custom_dot_offset() {
    use rnl_cli::solve::{solve, SolveOptions};
    let original = include_str!("junction_completion_fixtures/no-existing-dot.sp");
    let text = original.replace(
        "{ {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") }",
        "{ ?p ? @DISTANCE(\"left-to-right\", exact=2); {? ?p} @ALIGN(\"y\") }",
    );
    assert_ne!(text, original, "fixture must actually change the definition");
    let files = MemoryProvider::new().with("main.sp", &text);
    let options = SolveOptions { std: false, ..Default::default() };
    let result = solve(Path::new("main.sp"), &files, &options).expect("solve and custom junction completion");
    let file = result.files.iter().find(|file| file.path == "main.sp").expect("written main");
    assert_eq!(completion_suffix(&file.text, "$DOT"), "@PLACE(2, 0)", "{}", file.text);
}

#[test]
fn actual_solve_with_official_standard_dot() {
    use rnl_cli::solve::{solve, SolveOptions};
    let original = include_str!("junction_completion_fixtures/no-existing-dot.sp");
    let text = original
        .replace("\r\n", "\n")
        .replace(
            "@SOLVER-CONFIG(junction=\"$DOT\")",
            "@USE-PACKAGE(name=\"rnl-std-basic\",version=\"0.4\"); @SOLVER-CONFIG(junction=\"$JUNCTION\")",
        )
        .replace(
            "*   $DOT_SYMBOL = @DEFINE-SYMBOL(box=[0,0,0,0]) { (0,0) @CIRCLE(radius=0.4, fill=\"linecolor\") };\n",
            "",
        )
        .replace("*   $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") };\n", "")
        .replace("*   # @SELECT-USAGE $DOT => @SET-SYMBOL $DOT_SYMBOL;\n", "");
    assert!(!text.contains("$DOT"));
    let files = MemoryProvider::new().with("main.sp", &text);
    let result = solve(Path::new("main.sp"), &files, &SolveOptions { std: false, ..Default::default() })
        .expect("official junction solve");
    let file = result.files.iter().find(|file| file.path == "main.sp").expect("written main");
    assert_eq!(completion_suffix(&file.text, "$JUNCTION"), "@PLACE(0, 0)", "{}", file.text);
}

#[test]
fn two_new_junctions_are_both_placed_without_extra_backend() {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    use std::cell::Cell;
    let text = "* two branches\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a 0 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") } }\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let calls = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|_, _| {
            calls.set(calls.get() + 1);
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        branches: vec![
                            Branch { at: (0., 0.), junction: true },
                            Branch { at: (3., 4.), junction: true },
                        ],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("two-junction completion");
    assert_eq!(calls.get(), 1);
    let record = &result.levels[0].record;
    assert!(record.contains("$DOT @ATTACH %rt1 => @PLACE(0, 0)"), "{record}");
    assert!(record.contains("$DOT @ATTACH %rt2 => @PLACE(3, 4)"), "{record}");
}

#[test]
fn unrelated_unsolved_constraint_does_not_block_new_junction() {
    let record = run(r#"$DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") }; %u=(_, _); %v=(_, _); {%u %v} @ALIGN("x")"#).expect("unrelated free points are allowed");
    assert!(record.contains("$DOT @ATTACH %rt1 => @PLACE(0, 0)"), "{record}");
}

#[test]
fn repeated_authored_lines_keep_distinct_sources_after_old_record_removal() {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    let text = r#"* source map
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a 0 1k
* @RNL(type="generated",priority="high",author="old",hash="old") { %old=(5,5) }
* @RNL { $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") } }
* @RNL { #R1 @PLACE(1,2) }
* @RNL { #R1 @PLACE(1,2) }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|p, _| {
            let kept = (0..p.facts.len()).collect();
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        branches: vec![Branch { at: (0., 0.), junction: true }],
                        ..Default::default()
                    },
                    report: rnl_solver::SolveReport { kept, ..Default::default() },
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("source mapping preserves repeated authored lines");
    assert!(result.levels[0].record.contains("@PLACE(0, 0)"));
}

#[test]
fn existing_physical_pin_target_keeps_its_identity() {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    use rnl_runtime::problem::PointRef;
    let text = include_str!("junction_completion_fixtures/no-existing-dot.sp");
    let files = MemoryProvider::new().with("main.sp", text);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|p, _| {
            let pin = p
                .nets
                .iter()
                .flat_map(|n| &n.pins)
                .find(|pin| matches!(pin, PointRef::Pin { object, .. } if p.objects[*object].label == "#R1"))
                .expect("R1 physical pin")
                .clone();
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution { junctions: vec![pin], ..Default::default() },
                    ..Default::default()
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("junction on named physical pin");
    let record = &result.levels[0].record;
    assert!(record.contains("$DOT @ATTACH #R1:"), "{record}");
    assert!(record.contains("=> @PLACE(-6, 0)"), "{record}");
}

#[test]
fn all_junctions_share_one_global_interpretation_budget() {
    use std::cell::Cell;
    let text = "* four dots\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a 0 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") } }\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &original.problems[0];
    let solution = Solution {
        branches: (0..4).map(|i| Branch { at: (i as f64, 0.), junction: true }).collect(),
        ..Default::default()
    };
    let written = write_record(p, &solution, "test", "test");
    let formats = BTreeMap::new();
    let calls = Cell::new(0);
    let ctx = Context {
        entry: "main.sp",
        dialect: None,
        formats: &formats,
        files: &files,
        runtime: &opts,
        original: &original,
        problem: p,
        original_text: text,
        host_path: "main.sp",
        kept: &[],
        solution: &solution,
        tolerance: opts.tolerance,
        on_interpret: Some(&|| calls.set(calls.get() + 1)),
    };
    let result = complete_record(&ctx, &written).expect("batch completion");
    assert_eq!(result.trials, 1);
    assert_eq!(calls.get(), 2); // one baseline + one trial for all four
    assert_eq!(result.record.matches("@ATTACH").count(), 4);
}

#[test]
fn global_trial_cap_is_shared_across_many_junctions() {
    use std::cell::Cell;
    let text = "* bounded dots\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a 0 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { ?p ? @DISTANCE(\"left-to-right\",exact=2); ?p ? @DISTANCE(\"left-to-right\",exact=3); {? ?p} @ALIGN(\"y\") } }\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &original.problems[0];
    let solution = Solution {
        branches: (0..4).map(|i| Branch { at: (i as f64, 0.), junction: true }).collect(),
        ..Default::default()
    };
    let written = write_record(p, &solution, "test", "test");
    let formats = BTreeMap::new();
    let calls = Cell::new(0);
    let ctx = Context {
        entry: "main.sp",
        dialect: None,
        formats: &formats,
        files: &files,
        runtime: &opts,
        original: &original,
        problem: p,
        original_text: text,
        host_path: "main.sp",
        kept: &[],
        solution: &solution,
        tolerance: opts.tolerance,
        on_interpret: Some(&|| calls.set(calls.get() + 1)),
    };
    assert!(complete_record(&ctx, &written).is_err());
    assert!(calls.get() <= 65, "{} readings exceeded one baseline plus 64 trials", calls.get());
}

#[test]
fn external_selector_constraint_referencing_new_dot_must_be_decidable() {
    let check = r#"* external selected point
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a 0 1k
* @RNL { $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") }; %q=(_, _); %selected = # @SELECT-USAGE $DOT; {%selected %q} @ALIGN("x") }
* @RNL(type="generated",priority="high",author="test",hash="test") { %rt1=(0,0); $DOT @ATTACH %rt1 => @PLACE(0,0) }
.end
"#;
    let provider = MemoryProvider::new().with("main.sp", check);
    let interpreted = interpret_file(
        Path::new("main.sp"),
        None,
        &provider,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(
        !interpreted.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        interpreted.diagnostics()
    );
    assert!(
        interpreted.model.levels[0].constraints.iter().any(|c| c.status == "unsolved"),
        "{:?}",
        interpreted.model.levels[0].constraints
    );
    let result = run(
        r#"$DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") }; %q=(_, _); %selected = # @SELECT-USAGE $DOT; {%selected %q} @ALIGN("x")"#,
    );
    assert!(result.is_err(), "external constraint references the new dot and an unresolved point: {result:?}");
}

#[test]
fn effective_solver_tolerance_is_not_relaxed_by_readback_default() {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    fn attempt(tolerance: &str) -> Result<rnl_cli::solve::SolveRun, String> {
        let text = format!(
            r#"* strict tolerance
* @RNL(type="header") {{ @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT", tolerance={tolerance}) }}
R1 a 0 1k
* @RNL {{ $DOT = @DEFINE(roles=[["p","point"]]) {{ ?p ? @DISTANCE("left-to-right",exact=2); ?p ? @DISTANCE("left-to-right",exact=2.0000001); {{? ?p}} @ALIGN("y") }} }}
.end
"#
        );
        let files = MemoryProvider::new().with("main.sp", &text);
        solve_using(
            Path::new("main.sp"),
            &files,
            &SolveOptions { std: false, ..Default::default() },
            &|_| true,
            &|_, _| {
                Ok(BackendResult {
                    solved: rnl_solver::Solved {
                        solution: Solution {
                            branches: vec![Branch { at: (0., 0.), junction: true }],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    author: "test".into(),
                    report_available: true,
                })
            },
        )
    }
    // strict tolerance does not accept a 1e-7 discrepancy: the dot is left out, the rest written
    let strict = attempt("0").expect("the level is written without the dot");
    assert!(!strict.levels[0].record.contains("$DOT @ATTACH"), "{}", strict.levels[0].record);
    assert_eq!(strict.levels[0].written.unsolved[0].code, rnl_runtime::diag::codes::UNSOLVED_JUNCTION_UNPLACED);
    let normal = attempt("0.000001").expect("normal tolerance permits the discrepancy");
    assert!(normal.levels[0].record.contains("$DOT @ATTACH %rt1 => @PLACE"), "{}", normal.levels[0].record);
}

#[test]
fn following_text_is_checked_at_its_drawn_position_with_effective_tolerance() {
    use rnl_runtime::{problem::ObjectKind, solution::Placement};
    let text = include_str!("junction_completion_fixtures/no-existing-dot.sp")
        .replace("*   %p = (0,0);", "*   #R1 @TEXT(\"label\", key=\"note\", dx=0.1, dy=0.2);\n*   %p = (0,0);");
    let files = MemoryProvider::new().with("main.sp", &text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &original.problems[0];
    let object = p.objects.iter().position(|o| o.kind == ObjectKind::Text).unwrap();
    let formats = BTreeMap::new();
    for (delta, tolerance, accepted) in [(0., 1e-9, true), (5e-8, 1e-9, false), (5e-8, 1e-6, true)] {
        let solution = Solution {
            placements: vec![Placement { object, x: Some(-6. + 0.1 + delta), y: Some(0.2), orient: None }],
            branches: vec![Branch { at: (0., 0.), junction: true }],
            ..Default::default()
        };
        let written = write_record(p, &solution, "test", "test");
        assert!(!written.junctions.is_empty());
        let ctx = Context {
            entry: "main.sp",
            dialect: None,
            formats: &formats,
            files: &files,
            runtime: &opts,
            original: &original,
            problem: p,
            original_text: &text,
            host_path: "main.sp",
            kept: &[],
            solution: &solution,
            tolerance,
            on_interpret: None,
        };
        let result = complete_record(&ctx, &written);
        assert_eq!(result.is_ok(), accepted, "delta={delta}, tolerance={tolerance}: {:?}", result.err());
    }
}

#[test]
fn a_new_dropped_constraint_is_not_excused_by_an_existing_drop() {
    use rnl_cli::solve::{solve_with_clock, SolveOptions};
    let source = include_str!("junction_completion_fixtures/no-existing-dot.sp")
        .replace(
            "*   %p = (0,0);",
            "*   {#R1 #R2} @ORIENT(\"R0\"); #R1 #R2 @ORIENT-RELATION(\"R180\",priority=\"low\");\n*   %p = (0,0);",
        )
        .replace(
            "*   %p = (0,0);",
            "*   %dots = # @SELECT-USAGE $DOT; %dots @ORIENT(\"R0\"); #R1 %dots @ORIENT-RELATION(\"R90\",priority=\"low\");\n*   %p = (0,0);",
        );
    let files = MemoryProvider::new().with("main.sp", &source);
    let options = SolveOptions {
        std: false,
        solver: rnl_solver::SolverOptions::default().with_config("layout=\"lines\",limit=\"seed\"").unwrap(),
        ..Default::default()
    };
    // the new drop is the dot's, as the constraint names the dots: the dot is left out
    let solved = solve_with_clock(Path::new("main.sp"), &files, &options, &rnl_solver::StoppedClock).unwrap();
    let level = &solved.levels[0];
    assert!(!level.record.contains("$DOT @ATTACH"), "{}", level.record);
    assert!(
        level.written.notes.iter().any(|n| n.contains("new or changed fact is unresolved or not satisfied")),
        "{:?}",
        level.written.notes
    );
}

#[test]
fn existing_assertion_violation_is_preserved_but_a_new_one_is_rejected() {
    use rnl_cli::solve::{solve_with_clock, SolveOptions};
    let base = include_str!("junction_completion_fixtures/no-existing-dot.sp")
        .replace("*   %p = (0,0);", "*   {#R1 #R2} @ASSERT-INSTANCE(same=[\"name\"]);\n*   %p = (0,0);");
    for introduced in [false, true] {
        let source = if introduced {
            base.replace("*   %p = (0,0);", "*   $CHECK = @DEFINE(roles=[\"trigger\",\"{members}\"]) { ?members @ASSERT-INSTANCE(same=[\"name\"]) };\n*   %dots = # @SELECT-USAGE $DOT; %dots(trigger) {#R1 #R2}(members) @AS $CHECK;\n*   %p = (0,0);")
        } else {
            base.clone()
        };
        let files = MemoryProvider::new().with("main.sp", &source);
        let opts = Options { std: false, generated: false, problems: true, ..Default::default() };
        let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
        assert!(
            !original.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
            "{:?}",
            original.diagnostics()
        );
        assert_eq!(
            original.diagnostics().iter().filter(|d| d.severity == rnl_runtime::diag::Severity::Violation).count(),
            1
        );
        let options = SolveOptions {
            std: false,
            solver: rnl_solver::SolverOptions::default().with_config("layout=\"lines\",limit=\"seed\"").unwrap(),
            ..Default::default()
        };
        let result = solve_with_clock(Path::new("main.sp"), &files, &options, &rnl_solver::StoppedClock);
        if introduced {
            // the violation the dot would bring is refused: the dot is left out
            let level = &result.unwrap().levels[0];
            assert!(!level.record.contains("$DOT @ATTACH"), "{}", level.record);
            assert!(
                level.written.notes.iter().any(|n| n.contains("trial interpretation has an error or violation")),
                "{:?}",
                level.written.notes
            );
        } else {
            let result = result.unwrap();
            let saved = MemoryProvider::new().with("main.sp", &result.files[0].text);
            let after =
                interpret_file(Path::new("main.sp"), None, &saved, &Options { generated: true, ..opts }).unwrap();
            assert_eq!(
                after.diagnostics().iter().filter(|d| d.severity == rnl_runtime::diag::Severity::Violation).count(),
                1
            );
        }
    }
}

#[test]
fn already_fixed_usage_keeps_original_position_without_place_rewrite() {
    let record = run(r#"$DOT = @DEFINE(roles=[["p","point"]]) {}; # @SELECT-USAGE $DOT => @PLACE(2,0)"#)
        .expect("external fixed own point");
    assert!(record.contains("$DOT @ATTACH %rt1"), "{record}");
    assert!(!record.contains("$DOT @ATTACH %rt1 => @PLACE"), "{record}");
}

#[test]
fn one_fixed_axis_is_not_overridden_when_other_is_aligned() {
    let record =
        run(r#"$DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("y") }; # @SELECT-USAGE $DOT => @PLACE(x=2)"#)
            .expect("fixed x and role-aligned y");
    assert!(record.contains("$DOT @ATTACH %rt1 => @PLACE(y=0)"), "{record}");
}

#[test]
fn one_fixed_axis_is_not_overridden_when_other_is_free() {
    let record = run(r#"$DOT = @DEFINE(roles=[["p","point"]]) {}; # @SELECT-USAGE $DOT => @PLACE(x=2)"#)
        .expect("fixed x and free y");
    assert!(record.contains("$DOT @ATTACH %rt1 => @PLACE(y=0)"), "{record}");
}

#[test]
fn self_relative_place_in_definition_fails_after_application() {
    let result = run(r#"$DOT = @DEFINE(roles=[["p","point"]]) { ? @PLACE(2,0) }"#);
    assert!(result.is_err(), "self-relative PLACE fails when the definition is applied");
}

#[test]
fn actual_solve_preserves_external_fixed_axes() {
    use rnl_cli::solve::{solve, SolveOptions};
    let original = include_str!("junction_completion_fixtures/no-existing-dot.sp");
    for (selector, expected) in [("@PLACE(2,0)", ""), ("@PLACE(x=2)", "@PLACE(y=0)")] {
        let text =
            original.replace("{ {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") }", "{ {? ?p} @ALIGN(\"y\") }").replace(
                "*   # @SELECT-USAGE $DOT => @SET-SYMBOL $DOT_SYMBOL;",
                &format!(
                    "*   # @SELECT-USAGE $DOT => @SET-SYMBOL $DOT_SYMBOL;\n*   # @SELECT-USAGE $DOT => {selector};"
                ),
            );
        // Full fixed (2,0) is compatible with the body's y alignment.
        assert_ne!(text, original);
        let files = MemoryProvider::new().with("main.sp", &text);
        let mut options = SolveOptions { std: false, ..Default::default() };
        options.solver.run_limit = Some(rnl_solver::RunLimit::Work(5000));
        let result = solve(Path::new("main.sp"), &files, &options).expect("actual solver fixed axis");
        let record = &result.levels[0].record;
        assert_eq!(completion_suffix(record, "$DOT"), expected, "{record}");
    }
}

/// The level `four dots` of [`all_junctions_share_one_global_interpretation_budget`], its four dots
/// placed by [`complete_what_can_be_by`] with `deadline`, and the readings of the document it took.
fn four_dots_by(
    deadline: Option<&rnl_cli::junction_completion_driver::Deadline>,
) -> (Result<rnl_cli::junction_completion_driver::Partial, String>, String, usize) {
    use rnl_cli::junction_completion_driver::{complete_what_can_be_by, only};
    use std::cell::Cell;
    let text = "* four dots\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a 0 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") } }\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &original.problems[0];
    let solution = Solution {
        branches: (0..4).map(|i| Branch { at: (i as f64, 0.), junction: true }).collect(),
        ..Default::default()
    };
    let written = write_record(p, &solution, "test", "test");
    let formats = BTreeMap::new();
    let calls = Cell::new(0);
    let ctx = Context {
        entry: "main.sp",
        dialect: None,
        formats: &formats,
        files: &files,
        runtime: &opts,
        original: &original,
        problem: p,
        original_text: text,
        host_path: "main.sp",
        kept: &[],
        solution: &solution,
        tolerance: opts.tolerance,
        on_interpret: Some(&|| calls.set(calls.get() + 1)),
    };
    let result = complete_what_can_be_by(&ctx, &written, deadline);
    (result, only(&written, &[]).record, calls.get())
}

/// A level whose time ran out before its dots were tried: no dot is tried, each is left out for the
/// time, not for want of a place, and the record without them is checked all the same and written.
#[test]
fn dots_not_tried_before_the_deadline_are_left_out_for_the_time() {
    use rnl_cli::junction_completion_driver::{Deadline, RAN_OUT};
    let passed = Deadline::after(std::time::Duration::ZERO);
    let (result, bare, readings) = four_dots_by(Some(&passed));
    let partial = result.expect("the record without the dots reads back");
    assert_eq!(partial.trials, 0);
    assert_eq!(partial.dropped.iter().map(|d| d.0).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert!(partial.dropped.iter().all(|d| d.1 == RAN_OUT), "{:?}", partial.dropped);
    assert_eq!(partial.record, bare);
    assert_eq!(partial.record.matches("@ATTACH").count(), 0);
    // the check of the record without any dot only: its reading and its one trial
    assert_eq!(readings, 2);
}

/// With no deadline, or one far off, the dots are placed as they always were.
#[test]
fn without_a_deadline_the_dots_are_placed_as_before() {
    use rnl_cli::junction_completion_driver::Deadline;
    let (none, _, readings) = four_dots_by(None);
    let none = none.expect("placed");
    assert!(none.dropped.is_empty());
    assert_eq!((none.trials, readings), (1, 2));
    assert_eq!(none.record.matches("@ATTACH").count(), 4);
    let far = Deadline::after(std::time::Duration::from_secs(3600));
    let (late, _, _) = four_dots_by(Some(&far));
    let late = late.expect("placed");
    assert_eq!((late.record, late.trials, late.dropped.len()), (none.record, none.trials, 0));
}

/// A level of four dots solved by a backend that takes `seconds` and stops for `reason`, under a
/// safety limit of 1 s: the options the backend was given, the run, and its wall time.
fn four_dots_stopped(
    seconds: f64,
    reason: rnl_solver::StopReason,
) -> (rnl_solver::SolverOptions, rnl_cli::solve::SolveRun, f64) {
    use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
    use std::cell::RefCell;
    let text = "* four dots\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a 0 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") } }\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let solver =
        rnl_solver::SolverOptions::default().with_config(r#"limit="work", work=1000, safety_seconds=1"#).unwrap();
    let given = RefCell::new(None);
    let start = std::time::Instant::now();
    let run = solve_using(
        Path::new("main.sp"),
        &files,
        // the whole file's time, which a level of a limit of work does not share, so that the run is
        // on the wall clock, which the limits are read on
        &SolveOptions { std: false, solver, total_seconds: Some(3600.), ..Default::default() },
        &|_| true,
        &|p, options| {
            *given.borrow_mut() = Some(options.for_problem(p).0);
            std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
            let mut solved = rnl_solver::Solved {
                solution: Solution {
                    branches: (0..4).map(|i| Branch { at: (i as f64, 0.), junction: true }).collect(),
                    ..Default::default()
                },
                ..Default::default()
            };
            solved.report.budget =
                Some(rnl_solver::BudgetReport { limit: rnl_solver::RunLimit::Work(1000), work: 1000, reason });
            Ok(BackendResult { solved, author: "test".into(), report_available: true })
        },
    )
    .expect("the level is written");
    let wall = start.elapsed().as_secs_f64();
    (given.into_inner().expect("the backend ran"), run, wall)
}

/// A level its safety limit stopped, its search using all of the limit: its dots are not tried
/// once the limit is past, each is left out for the time, and the level is written. The run ends
/// within the limit, the check of the record without any dot, and the write: on this level, well
/// within a second of the limit, where all four dots tried one by one would take longer.
#[test]
fn a_level_stopped_by_its_safety_limit_ends_near_it() {
    let (given, run, wall) = four_dots_stopped(1.0, rnl_solver::StopReason::Safety);
    // the search was given less than the limit: the rest is kept for the dots and the write
    let safety = given.safety_seconds.expect("a safety limit");
    assert!((0.75..1.0).contains(&safety), "{safety}");
    let level = &run.levels[0];
    assert!(level.written.notes.iter().any(|n| n.contains("kept for junction placement")), "{:?}", level.written.notes);
    let time: Vec<_> =
        level.written.unsolved.iter().filter(|u| u.reason.contains("ran out before it was tried")).collect();
    assert_eq!(time.len(), 4, "{:?}", level.written.unsolved);
    assert!(time.iter().all(|u| u.code == rnl_runtime::diag::codes::UNSOLVED_JUNCTION_UNPLACED));
    assert!(!level.record.contains("$DOT @ATTACH"), "{}", level.record);
    assert!(level.written.notes.iter().any(|n| n.contains("0 candidate trials")), "{:?}", level.written.notes);
    assert!(wall < 1.0 + 1.0, "{wall}");
}

/// A level stopped by its work, or by its time, is replayed by its work, which places its dots with
/// no time to them: it places them all as well, however long its search took.
#[test]
fn a_level_not_stopped_by_its_safety_limit_places_all_its_dots() {
    for reason in [rnl_solver::StopReason::Work, rnl_solver::StopReason::Time] {
        let (_, run, _) = four_dots_stopped(1.0, reason);
        let level = &run.levels[0];
        assert!(level.written.unsolved.is_empty(), "{reason:?}: {:?}", level.written.unsolved);
        assert_eq!(level.record.matches("$DOT @ATTACH").count(), 4, "{reason:?}: {}", level.record);
    }
}

#[test]
fn objects_lost_on_reading_the_record_back_are_named() {
    use rnl_cli::junction_completion_driver::{complete_what_can_be, LOST};
    // rules write a text and attach a $M at each pin no wire is on; the record wires #R1:1, as a
    // record of the solver wires the pin of a stub of the fill that a rule of the file selected
    let text = "* lost\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }\nR1 a 0 1k\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") }; #R1 @SELECT-PIN(wired=false) => @TEXT(\"${net}\", key=\"k\"); $M = @DEFINE(roles=[[\"p\",\"pin\"]]) {}; #R1 @SELECT-PIN(wired=false) => $M @ATTACH }\n.end\n";
    let files = MemoryProvider::new().with("main.sp", text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let interpreted = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &interpreted.problems[0];
    let mut written = write_record(
        p,
        &Solution { branches: vec![Branch { at: (0., 0.), junction: true }], ..Default::default() },
        "test",
        "test",
    );
    // its last statement, after the dot's, whose place in the record stays
    let end = written.record.rfind("\n}").expect("the end of the record");
    written.record.insert_str(end, ";\n  &a @WIRE {#R1:1 (5, 5)}");
    let formats = BTreeMap::new();
    let ctx = Context {
        entry: "main.sp",
        dialect: None,
        formats: &formats,
        files: &files,
        runtime: &opts,
        original: &interpreted,
        problem: p,
        original_text: text,
        host_path: "main.sp",
        kept: &[],
        solution: &Solution::default(),
        tolerance: opts.tolerance,
        on_interpret: None,
    };
    let why = complete_record(&ctx, &written).map(|c| c.record).unwrap_err();
    assert_eq!(why, format!("#R1:p::k; a usage of $M, made by the rule at main.sp:4:220 are {LOST}"));
    // the same without the dots: what is lost is not the dots', and said once
    let why = complete_what_can_be(&ctx, &written).map(|c| c.record).unwrap_err();
    assert_eq!(why, format!("#R1:p::k; a usage of $M, made by the rule at main.sp:4:220 are {LOST}"));
}
