//! Junction completion must number trial records before interpreting anonymous placements.
use std::{cell::Cell, path::Path};

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

const INPUT: &str = r#"* anonymous usages and offset junction
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL {
*   $MARK = @DEFINE(roles=[["n","net"]]) {};
*   $MARK_SYMBOL = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=0.25) };
*   &a @AS $MARK => @SET-SYMBOL $MARK_SYMBOL;
*   &b @AS $MARK => @SET-SYMBOL $MARK_SYMBOL;
*   $DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right",exact=2); {? ?p} @ALIGN("y") }
* }
.end
"#;

fn coordinates(position: &Position) -> Option<(f64, f64)> {
    match position {
        Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } => Some((*x, *y)),
        _ => None,
    }
}

#[test]
fn numbered_anonymous_usages_survive_junction_completion_and_reinterpretation() {
    let files = MemoryProvider::new().with("main.sp", INPUT);
    let calls = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|p, _| {
            calls.set(calls.get() + 1);
            let marks: Vec<_> = p.usages.iter().filter(|u| u.definition == "$MARK").collect();
            assert_eq!(marks.len(), 2);
            let placements = marks
                .iter()
                .enumerate()
                .map(|(i, u)| {
                    let object = u.object.expect("symbol-bearing usage must have a problem object");
                    assert!(matches!(&p.objects[object].reference, Reference::Number { path, .. } if path.is_empty()));
                    let (x, y) = if i == 0 { (7.0, 11.0) } else { (13.0, -4.0) };
                    Placement { object, x: Some(x), y: Some(y), orient: None }
                })
                .collect();
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        placements,
                        branches: vec![Branch { at: (3.0, 4.0), junction: true }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("trial interpretation must retain numbered anonymous placements");
    assert_eq!(calls.get(), 1);
    let written = &result.files.iter().find(|f| f.path == "main.sp").expect("written file").text;
    assert!(written.contains("numbering=\""), "{written}");
    assert!(written.contains("%#"), "{written}");
    let saved = MemoryProvider::new().with("main.sp", written);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &saved,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(
        !read.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::NUMBERING_RECORD_STALE),
        "{:?}",
        read.diagnostics()
    );
    let p = &read.problems[0];
    let marks: Vec<_> = p.usages.iter().filter(|u| u.definition == "$MARK").collect();
    assert_eq!(marks.len(), 2);
    assert_eq!(coordinates(&p.objects[marks[0].object.unwrap()].position), Some((7.0, 11.0)));
    assert_eq!(coordinates(&p.objects[marks[1].object.unwrap()].position), Some((13.0, -4.0)));
    let dot = p.usages.iter().find(|u| u.definition == "$DOT").expect("new junction usage");
    assert_eq!(coordinates(&p.objects[dot.object.unwrap()].position), Some((5.0, 4.0)));

    // A deliberately wrong check must still make the numeric placements stale on re-read.
    let start = written.find("numbering=\"").unwrap() + "numbering=\"".len();
    let end = start + written[start..].find('"').unwrap();
    let old = &written[start..end];
    let bad = if old == "0000000000000000" { "1111111111111111" } else { "0000000000000000" };
    let mut corrupted = written.clone();
    corrupted.replace_range(start..end, bad);
    let stale_files = MemoryProvider::new().with("main.sp", &corrupted);
    let stale = interpret_file(
        Path::new("main.sp"),
        None,
        &stale_files,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(stale.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::NUMBERING_RECORD_STALE));
    let p = &stale.problems[0];
    let stale_marks: Vec<_> = p.usages.iter().filter(|u| u.definition == "$MARK").collect();
    assert_eq!(stale_marks.len(), 2);
    assert_ne!(coordinates(&p.objects[stale_marks[0].object.unwrap()].position), Some((7.0, 11.0)));
    assert_ne!(coordinates(&p.objects[stale_marks[1].object.unwrap()].position), Some((13.0, -4.0)));
}

#[test]
fn headerless_included_file_inherits_04_and_writes_numbered_usages_in_two_levels() {
    let main = r#"* include junctions
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
.include "child.inc"
X1 a b first
X2 a b second
.end
"#;
    let child = r#"* deliberately has no header
.subckt first a b
R1 a b 1k
* @RNL {
* @SOLVER-CONFIG(junction="$DOT");
* $MARK = @DEFINE(roles=[["n","net"]]) {};
* $S = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=0.25) };
* &a @AS $MARK => @SET-SYMBOL $S;
* $DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right",exact=2); {? ?p} @ALIGN("y") }
* }
.ends first
.subckt second a b
R2 a b 1k
* @RNL {
* @SOLVER-CONFIG(junction="$DOT");
* $MARK = @DEFINE(roles=[["n","net"]]) {};
* $S = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=0.25) };
* &b @AS $MARK => @SET-SYMBOL $S;
* $DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right",exact=2); {? ?p} @ALIGN("y") }
* }
.ends second
"#;
    let files = MemoryProvider::new().with("main.sp", main).with("child.inc", child);
    let solved_levels = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|p, _| {
            let marks: Vec<_> = p.usages.iter().filter(|u| u.definition == "$MARK").collect();
            if marks.is_empty() {
                return Ok(BackendResult {
                    solved: rnl_solver::Solved::default(),
                    author: "test".into(),
                    report_available: true,
                });
            }
            assert_eq!(marks.len(), 1);
            let config = p.junction.effective();
            assert_eq!(config.definition.as_deref(), Some("$DOT"), "level {}: {config:?}", p.name);
            assert!(config.invalid.is_none(), "level {}: {config:?}", p.name);
            let object = marks[0].object.expect("symbol-bearing anonymous usage");
            assert!(matches!(&p.objects[object].reference, Reference::Number { path, .. } if path.is_empty()));
            let i = solved_levels.get();
            solved_levels.set(i + 1);
            let x = 10.0 + 10.0 * i as f64;
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        placements: vec![Placement { object, x: Some(x), y: Some(7.0), orient: None }],
                        branches: vec![Branch { at: (x, 4.0), junction: true }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("headerless included levels must accept numbered trial records");
    assert_eq!(solved_levels.get(), 2);
    let dot_levels: Vec<_> = result
        .levels
        .iter()
        .filter(|level| level.problem.usages.iter().any(|usage| usage.definition == "$MARK"))
        .collect();
    assert_eq!(dot_levels.len(), 2);
    for level in dot_levels {
        assert!(
            level.record.contains("$DOT @ATTACH"),
            "level {} record: {} warnings: {:?}",
            level.problem.name,
            level.record,
            level.written.warnings
        );
    }
    let written_child = &result.files.iter().find(|f| f.path == "child.inc").expect("updated include").text;
    assert!(written_child.contains("@VERSION(\"0.4\")"), "{written_child}");
    assert!(written_child.contains("type=\"header\""), "{written_child}");
    assert_eq!(written_child.matches("numbering=\"").count(), 2, "{written_child}");
    let written_main = result.files.iter().find(|f| f.path == "main.sp").map_or(main, |f| f.text.as_str());
    let saved = MemoryProvider::new().with("main.sp", written_main).with("child.inc", written_child);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &saved,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(
        !read.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::NUMBERING_RECORD_STALE),
        "{:?}",
        read.diagnostics()
    );
    let mut seen = Vec::new();
    for p in &read.problems {
        for mark in p.usages.iter().filter(|u| u.definition == "$MARK") {
            let pos = coordinates(&p.objects[mark.object.unwrap()].position).expect("placed anonymous mark");
            let dot = p.usages.iter().find(|u| u.definition == "$DOT").expect("junction in same level");
            let dot_pos = coordinates(&p.objects[dot.object.unwrap()].position).expect("placed junction");
            assert_eq!(dot_pos, (pos.0 + 2.0, 4.0));
            assert_eq!(pos.1, 7.0);
            seen.push(pos.0 as i32);
        }
    }
    seen.sort_unstable();
    assert_eq!(seen, [10, 20]);
}
