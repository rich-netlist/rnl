//! Integration coverage for generated identities through actual record write/read.
use rnl_cli::{
    interpret_file,
    solve::{solve_using, BackendResult, SolveOptions},
};
use rnl_runtime::{
    problem::{Axis, FactKind, PointRef, Position, Reference, Slot},
    solution::{Branch, Placement, Solution},
    source::MemoryProvider,
    Options,
};
use std::{cell::Cell, path::Path};

fn fixture(extra: &str, dot: &str) -> String {
    format!(
        r#"* identity edges
* @RNL(type="header") {{ @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }}
R1 a b 1k
* @RNL {{
* $MARK = @DEFINE(roles=[["x","instance"]]) {{}};
* $S = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=0.25) }};
* #R1 @PLACE(0,0);
* # @SELECT-INSTANCE(name="R1") => @AS $MARK;
* # @SELECT-USAGE $MARK => @SET-SYMBOL $S;
* $DOT = @DEFINE(roles=[["p","point"]]) {{ {dot} }};
* {extra}
* }}
.end
"#
    )
}
const DOT: &str = r#"?p ? @DISTANCE("left-to-right",exact=2); {? ?p} @ALIGN("y")"#;

fn solve_case(text: &str, include_nested: bool) -> (String, usize) {
    let files = MemoryProvider::new().with("main.sp", text);
    let calls = Cell::new(0);
    let result = solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|p, _| {
            calls.set(calls.get() + 1);
            let mark = p.usages.iter().find(|u| u.definition == "$MARK").unwrap();
            assert!(mark.rule_made);
            let mut placements =
                vec![Placement { object: mark.object.unwrap(), x: Some(7.), y: Some(11.), orient: None }];
            if include_nested {
                let index = p
                    .objects
                    .iter()
                    .position(|o| matches!(o.reference, Reference::GeneratedNumber { .. }))
                    .expect("fixture must produce an anonymous generated object");
                placements.push(Placement { object: index, x: Some(9.), y: Some(11.), orient: None });
            }
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        placements,
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
    .expect("completion must preserve written identities");
    let file = result.files.iter().find(|f| f.path == "main.sp").unwrap().text.clone();
    (file, calls.get())
}
fn reread(text: &str) -> rnl_runtime::Interpretation {
    let files = MemoryProvider::new().with("main.sp", text);
    interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn fixed(p: &rnl_runtime::problem::LevelProblem, i: usize) -> Option<(f64, f64)> {
    match &p.objects[i].position {
        Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } => Some((*x, *y)),
        _ => None,
    }
}

#[test]
fn generated_name_collision_and_unwritten_unrelated_usage_survive() {
    let text = fixture(
        r#"%mark_R1=(22,23); $OTHER = @DEFINE(roles=[["x","instance"]]) {}; # @SELECT-INSTANCE(name="R1") => @AS $OTHER"#,
        DOT,
    );
    let (out, calls) = solve_case(&text, false);
    assert_eq!(calls, 1);
    assert!(out.contains("%mark_R1_2 = $MARK @ATTACH #R1"), "{out}");
    let result = reread(&out);
    assert!(
        !result.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        result.diagnostics()
    );
    let p = &result.problems[0];
    let mark = p.usages.iter().find(|u| u.definition == "$MARK").unwrap();
    assert_eq!(mark.reference, Reference::Name("%mark_R1_2".into()));
    assert_eq!(fixed(p, mark.object.unwrap()), Some((7., 11.)));
    assert!(p.usages.iter().any(|u| u.definition == "$OTHER"));
}

#[test]
fn nested_anonymous_generated_object_keeps_its_numbered_identity() {
    let text = fixture("", DOT).replace("$MARK = @DEFINE(roles=[[\"x\",\"instance\"]]) {};", "$INNER = @DEFINE(roles=[[\"x\",\"instance\"]]) { (_,_) @TEXT(\"nested\") }; $MARK = @DEFINE(roles=[[\"x\",\"instance\"]]) { ?x @AS $INNER };");
    let (out, calls) = solve_case(&text, true);
    assert_eq!(calls, 1);
    assert!(out.contains("numbering="), "{out}");
    let result = reread(&out);
    assert!(
        !result.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        result.diagnostics()
    );
    assert!(result.problems[0].objects.iter().any(|o| matches!(o.reference, Reference::Number { .. })
        && matches!(
            o.position,
            Position::Own { x: Slot::Fixed { value: 9., .. }, y: Slot::Fixed { value: 11., .. } }
        )));
}

#[test]
fn conflicting_completion_leaves_the_dot_out_and_replaces_the_old_record() {
    let text = fixture("", r#"?p ? @DISTANCE("left-to-right",exact=2); ?p ? @DISTANCE("left-to-right",exact=3)"#)
        .replace(
            ".end",
            "* @RNL(type=\"generated\",priority=\"high\",author=\"old\",hash=\"old\") { %old=(5,5) }\n.end",
        );
    let files = MemoryProvider::new().with("main.sp", &text);
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
                        branches: vec![Branch { at: (3., 4.), junction: true }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                author: "test".into(),
                report_available: true,
            })
        },
    );
    let result = result.expect("the level is written without the dot");
    assert_eq!(calls.get(), 1);
    let record = &result.levels[0].record;
    assert!(!record.contains("$DOT @ATTACH") && record.contains("%rt1 = (3, 4)"), "{record}");
    let written = &result.files.iter().find(|file| file.path == "main.sp").expect("written main").text;
    assert!(!written.contains("%old=(5,5)"), "{written}");
}

#[test]
fn physical_pin_role_and_kept_alignment_survive_materialization() {
    let extra = r#"$P = @DEFINE-SYMBOL { %p=(1,0) @PIN(terminal=1,direction="right") }; #R1 @SET-SYMBOL $P; $PINMARK = @DEFINE(roles=[["p","point"]]) { {?p (1,0)} @ALIGN("y") }; #R1::p @AS $PINMARK => @PLACE(0,0)"#;
    let original = fixture(extra, DOT);
    let before = reread(&original);
    let (out, calls) = solve_case(&original, false);
    assert_eq!(calls, 1);
    let read = reread(&out);
    assert!(
        !read.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        read.diagnostics()
    );
    let p = &read.problems[0];
    let owner = p.objects.iter().position(|o| o.label == "#R1").unwrap();
    assert_eq!(fixed(p, owner), Some((0., 0.)));
    let pinmark = p.usages.iter().position(|x| x.definition == "$PINMARK").unwrap();
    assert!(p.facts.iter().any(|fact| fact.chain.contains(&pinmark)
        && matches!(&fact.kind, FactKind::Align { axis: Axis::Y, members } if members.iter().any(|point|
            matches!(point, PointRef::PinPosition { object, pin: 0, reference: Reference::Name(name) }
                if *object == owner && name == "#R1::p::position")))));
    let old = &before.problems[0];
    let old_owner = old.objects.iter().position(|o| o.label == "#R1").unwrap();
    let old_pin = &old.symbols[old.objects[old_owner].symbol.unwrap()].pins[0];
    let new_pin = &p.symbols[p.objects[owner].symbol.unwrap()].pins[0];
    assert_eq!(old_pin.identity, new_pin.identity);
    assert_eq!(old_pin.pos, new_pin.pos);
    assert_eq!(old_pin.terminals, new_pin.terminals);
    assert_eq!(old_pin.dir, Some(0));
    assert_eq!(old_pin.dir, new_pin.dir);
}
