//! Real rule-generated usage writeback across junction completion.
use rnl_cli::{
    interpret_file,
    solve::{solve_using, BackendResult, SolveOptions},
};
use rnl_runtime::{
    problem::{Position, Reference, Slot},
    solution::{Branch, Placement, Solution},
    source::MemoryProvider,
    Options,
};
use std::{cell::Cell, path::Path};

#[test]
fn rule_generated_usage_placement_survives_materialization_and_dot_completion() {
    let text = r#"* generated identity
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL {
* $MARK = @DEFINE(roles=[["x","instance"]]) {};
* $S = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=0.25) };
* #R1 @PLACE(0,0);
* # @SELECT-INSTANCE(name="R1") => @AS $MARK;
* # @SELECT-USAGE $MARK => @SET-SYMBOL $S;
* $DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right",exact=2); {? ?p} @ALIGN("y") }
* }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let calls = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|p, _| {
            calls.set(calls.get() + 1);
            let mark = p.usages.iter().find(|u| u.definition == "$MARK").expect("generated usage");
            assert!(mark.rule_made);
            let object = mark.object.expect("symbol-bearing usage");
            assert!(matches!(p.objects[object].reference, Reference::Generated { .. }));
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        placements: vec![Placement { object, x: Some(7.), y: Some(11.), orient: None }],
                        branches: vec![Branch { at: (3., 4.), junction: true }],
                        ..Default::default()
                    },
                    report: rnl_solver::SolveReport { kept: (0..p.facts.len()).collect(), ..Default::default() },
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("materialized generated usage must retain identity");
    assert_eq!(calls.get(), 1);
    let file = &result.files.iter().find(|f| f.path == "main.sp").unwrap().text;
    assert!(file.contains("$DOT @ATTACH"), "{file}");
    let saved = MemoryProvider::new().with("main.sp", file);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &saved,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    let p = &read.problems[0];
    let mark = p.usages.iter().find(|u| u.definition == "$MARK").unwrap();
    assert_eq!(mark.roles.len(), 1);
    assert_eq!(mark.roles[0].role, "x");
    assert!(
        matches!(&mark.roles[0].members[..], [rnl_runtime::problem::Filler::Object(i)] if p.objects[*i].label == "#R1")
    );
    assert!(matches!(mark.reference, Reference::Name(_)));
    let at = |object: usize| match &p.objects[object].position {
        Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } => Some((*x, *y)),
        _ => None,
    };
    assert_eq!(at(mark.object.unwrap()), Some((7., 11.)));
    let dot = p.usages.iter().find(|u| u.definition == "$DOT").unwrap();
    assert_eq!(at(dot.object.unwrap()), Some((5., 4.)));
}
