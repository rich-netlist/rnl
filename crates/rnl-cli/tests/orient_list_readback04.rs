//! Orientation alternatives remain one fact after solver writeback.
use rnl_cli::{
    interpret_file,
    solve::{solve, SolveOptions},
};
use rnl_runtime::{diag::codes, geom::Orient, source::MemoryProvider, Options};
use std::path::Path;

const INPUT: &str = r#"* orientation list
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
R2 c d 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (-3,0) @PIN(terminal=1,direction="left"); (3,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,1) @RECTANGLE };
* {#R1 #R2} @SET-SYMBOL $S;
* #R1 @PLACE(0,0); #R2 @PLACE(12,0); #R1:1::position @PLACE(3,0);
* #R1 @ORIENT(["R0","MY"]); #R2 @ORIENT("R0");
* #R1 #R2 @ORIENT-RELATION(["R0","MY"])
* }
.end
"#;
fn read(text: &str) -> rnl_runtime::Interpretation {
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", text),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn drops(r: &rnl_runtime::Interpretation) -> usize {
    r.diagnostics()
        .iter()
        .filter(|d| d.code == codes::CONFLICT_FACT_DISCARDED || d.code == codes::CONFLICT_FACT_DISCARDED_WITH_SETS)
        .count()
}
fn direction(r: &rnl_runtime::Interpretation) -> Orient {
    r.model.levels[0].items.iter().find(|o| o.id == "#R1").unwrap().orient
}
fn one_object(version: &str, facts: &str) -> String {
    let facts = facts.trim_end();
    format!(
        r#"* orientation facts
* @RNL(type="header") {{ @VERSION("{version}") }}
R1 a b 1k
* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=1) }}; #R1 @SET-SYMBOL $S; #R1 @PLACE(0,0); {facts}
* }}
.end
"#
    )
}
#[test]
fn nonfirst_choice_keeps_original_list_after_real_solve_and_readback() {
    let files = MemoryProvider::new().with("main.sp", INPUT);
    let run = solve(
        Path::new("main.sp"),
        &files,
        &SolveOptions {
            std: false,
            solver: rnl_solver::SolverOptions::default().with_config("work=10000").unwrap(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(run.levels[0].solved.report.kept.len(), 9);
    assert!(run.levels[0].solved.report.discarded.is_empty());
    let output = &run.files.iter().find(|f| f.path == "main.sp").unwrap().text;
    assert!(output.contains("@ORIENT(\"MY\")"), "{output}");
    let result = read(output);
    assert_eq!(direction(&result), Orient::MY);
    assert_eq!(drops(&result), 0, "{:?}", result.diagnostics());
}
#[test]
fn compatible_list_intersections_choose_a_common_orientation() {
    let src = INPUT.replace("#R1 @ORIENT([\"R0\",\"MY\"])", "#R1 @ORIENT([\"R0\",\"MY\"]); #R1 @ORIENT(\"MY\")");
    assert_ne!(src, INPUT);
    let r = read(&src);
    assert_eq!(direction(&r), Orient::MY);
    assert_eq!(drops(&r), 0, "{:?}", r.diagnostics());
}
#[test]
fn disjoint_lists_and_scalar_conflicts_remain_reported() {
    let disjoint = INPUT.replace("#R1 @ORIENT([\"R0\",\"MY\"])", "#R1 @ORIENT([\"R0\",\"R90\"]); #R1 @ORIENT(\"MY\")");
    let r = read(&disjoint);
    assert!(drops(&r) > 0, "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().any(|d| d.message.contains("orientation [R0, R90]")), "{:?}", r.diagnostics());
    let old = one_object("0.3", r#"#R1 @ORIENT("R0"); #R1 @ORIENT("MY")"#);
    let r = read(&old);
    assert_eq!(direction(&r), Orient::MY);
    assert!(drops(&r) > 0, "{:?}", r.diagnostics());
}

#[test]
fn three_pairwise_overlapping_lists_still_need_a_common_value() {
    let source = one_object("0.4", r#"#R1 @ORIENT(["R0","R90"]); #R1 @ORIENT(["R90","MY"]); #R1 @ORIENT(["R0","MY"])"#);
    let r = read(&source);
    assert_eq!(drops(&r), 1, "{:?}", r.diagnostics());
    // Without a solver result the highest-ranked list retains its first written direction.
    assert_eq!(direction(&r), Orient::R0);
}

#[test]
fn higher_priority_scalar_still_rejects_disjoint_lower_list() {
    let source = one_object("0.4", r#"#R1 @ORIENT("R90",priority="high"); #R1 @ORIENT(["R0","MY"])"#);
    let r = read(&source);
    assert_eq!(direction(&r), Orient::R90);
    assert_eq!(drops(&r), 1, "{:?}", r.diagnostics());
}

#[test]
fn truncated_conflict_enumeration_is_explicitly_incomplete() {
    let mut facts = String::from("#R1 @ORIENT(\"R90\");\n");
    for _ in 0..17 {
        facts.push_str("* #R1 @ORIENT([\"R0\",\"MY\"]);\n");
    }
    let source = one_object("0.4", &facts);
    let r = read(&source);
    assert_eq!(drops(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(direction(&r), Orient::R0);
    assert!(
        r.diagnostics()
            .iter()
            .any(|d| d.message.contains("orientation [R0, MY]") && d.message.contains("this list may not be complete")),
        "{:?}",
        r.diagnostics()
    );
}
