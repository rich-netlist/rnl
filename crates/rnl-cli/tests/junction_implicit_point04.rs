//! An already fixed rule-generated usage need not be materialized merely because a fact was kept.
use rnl_cli::{
    interpret_file,
    solve::{solve_using, BackendResult, SolveOptions},
};
use rnl_runtime::{
    problem::{FactKind, PointRef, Position, Slot},
    solution::{write_record, Branch, Solution},
    source::MemoryProvider,
    Options,
};
use std::{cell::Cell, path::Path};

const INPUT: &str = r#"* implicit kept usage
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL {
* $MARK = @DEFINE(roles=[["x","instance"]],parameters=[["label","mark"]]) {};
* $S = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=0.25) };
* # @SELECT-INSTANCE(name="R1") => @AS $MARK;
* # @SELECT-USAGE $MARK => @SET-SYMBOL $S;
* # @SELECT-USAGE $MARK => @PLACE(2,3);
* $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") }
* }
.end
"#;

#[test]
fn kept_fact_on_fixed_unwritten_rule_usage_survives_junction_completion() {
    let files = MemoryProvider::new().with("main.sp", INPUT);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &original.problems[0];
    let mark = p.usages.iter().find(|u| u.definition == "$MARK" && u.rule_made).expect("rule-created mark");
    let object = mark.object.expect("symbol-bearing usage");
    assert_eq!(mark.params.len(), 1, "the implicit usage must retain its string parameter");
    assert!(
        matches!(&p.objects[object].position,Position::Own{x:Slot::Fixed{value:x,..},y:Slot::Fixed{value:y,..}} if (*x,*y)==(2.,3.))
    );
    let kept: Vec<_> = p
        .facts
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            matches!(f.kind, FactKind::Place { at: PointRef::Origin(o), .. } if o == object).then_some(i)
        })
        .collect();
    assert!(!kept.is_empty(), "fixture must retain the placement of the implicit usage");
    let solution = Solution { branches: vec![Branch { at: (8., 8.), junction: true }], ..Default::default() };
    let written = write_record(p, &solution, "test", "test");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(
        !written.record.contains("$MARK @ATTACH"),
        "writer must not materialize the kept-only usage: {}",
        written.record
    );
    assert!(written.record.contains("$DOT @ATTACH"), "{}", written.record);
    let text = rnl_runtime::writeback::replace_generated_with_header(
        &original.doc,
        p.level,
        INPUT,
        &written.record,
        &p.version,
    );
    let readfiles = MemoryProvider::new().with("main.sp", &text);
    let reread = interpret_file(
        Path::new("main.sp"),
        None,
        &readfiles,
        &Options { std: false, problems: true, generated: true, ..Default::default() },
    )
    .unwrap();
    let ids = rnl_runtime::previous::document_ids(p, &reread);
    assert!(ids.objects[object].is_none(), "proof: old object has no direct materialized reference");
    let trial = &reread.problems[0];
    let still = trial.usages.iter().find(|u| u.definition == "$MARK" && u.rule_made).expect("rule recreates usage");
    let recreated = still.object.unwrap();
    assert!(
        matches!(&trial.objects[recreated].position,Position::Own{x:Slot::Fixed{value:x,..},y:Slot::Fixed{value:y,..}} if (*x,*y)==(2.,3.))
    );
    let calls = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|problem, _| {
            calls.set(calls.get() + 1);
            let object =
                problem.usages.iter().find(|u| u.definition == "$MARK" && u.rule_made).unwrap().object.unwrap();
            let kept = problem
                .facts
                .iter()
                .enumerate()
                .filter_map(|(i, f)| {
                    matches!(f.kind, FactKind::Place { at: PointRef::Origin(o), .. } if o == object).then_some(i)
                })
                .collect();
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: solution.clone(),
                    report: rnl_solver::SolveReport { kept, ..Default::default() },
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("kept fact on recreated fixed usage should be proved against its mapped drawing");
    assert_eq!(calls.get(), 1);
    assert!(result.levels[0].record.contains("$DOT @ATTACH"));
}

#[test]
fn requested_junction_on_generated_pin_materializes_its_owner() {
    use rnl_runtime::problem::PointRef;
    let text = INPUT.replace("(0,0) @CIRCLE(radius=0.25)", "%pin=(0,0) @PIN; (0,0) @CIRCLE(radius=0.25)");
    assert_ne!(text, INPUT);
    let files = MemoryProvider::new().with("main.sp", &text);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, problems: true, generated: false, ..Default::default() },
    )
    .unwrap();
    let p = &read.problems[0];
    let mark = p.usages.iter().find(|u| u.definition == "$MARK" && u.rule_made).unwrap();
    let object = mark.object.unwrap();
    let symbol = p.objects[object].symbol.expect("assigned symbol");
    assert_eq!(p.symbols[symbol].pins.len(), 1);
    let reference = p.objects[object].reference.then("::pin");
    let solution =
        Solution { junctions: vec![PointRef::SymbolPin { object, pin: 0, reference }], ..Default::default() };
    let written = write_record(p, &solution, "test", "test");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert_eq!(written.junctions.len(), 1, "{}", written.record);
    assert!(
        written.record.contains("$MARK @ATTACH"),
        "a junction reference must materialize its rule-created owner: {}",
        written.record
    );
}
