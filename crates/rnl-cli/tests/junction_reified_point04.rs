//! A dot on a pin owned by a rule-generated usage must follow that usage's writeback identity.
use rnl_cli::{
    interpret_file,
    solve::{solve_using, BackendResult, SolveOptions},
};
use rnl_runtime::{
    problem::{FactKind, PointRef, Position, Reference, Slot},
    solution::{Placement, Solution},
    source::MemoryProvider,
    Options,
};
use std::{cell::Cell, path::Path};

#[test]
fn dot_on_reified_usage_pin_keeps_its_physical_target() {
    let text = r#"* reified pin
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL {
* $MARK = @DEFINE(roles=[["x","instance"]]) {};
* $S = @DEFINE-SYMBOL { %p=(1,0) @PIN(direction="right") };
* #R1 @PLACE(0,0);
* # @SELECT-INSTANCE(name="R1") => @AS $MARK;
* # @SELECT-USAGE $MARK => @SET-SYMBOL $S;
* $DOT = @DEFINE(roles=[["p","point"]]) { ?p ? @DISTANCE("left-to-right",exact=2); {? ?p} @ALIGN("y") }
* }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let calls = Cell::new(0);
    let result = solve_using(Path::new("main.sp"), &files, &SolveOptions { std: false, ..Default::default() }, &|_| true, &|p, _| {
        calls.set(calls.get() + 1);
        let mark = p.usages.iter().find(|u| u.definition == "$MARK").expect("generated mark");
        assert!(mark.rule_made);
        let owner = mark.object.expect("drawn usage");
        assert!(matches!(p.objects[owner].reference, Reference::Generated { .. }));
        let symbol = &p.symbols[p.objects[owner].symbol.expect("mark symbol")];
        assert_eq!(symbol.pins.len(), 1);
        assert_eq!(symbol.pins[0].pos, (1., 0.));
        let point = PointRef::SymbolPin { object: owner, pin: 0, reference: p.objects[owner].reference.then("::p") };
        let kept = p.facts.iter().position(|f| matches!(&f.kind, FactKind::Place { at: PointRef::Origin(i), .. } if p.objects[*i].label == "#R1")).expect("authored R1 place fact");
        Ok(BackendResult {
            solved: rnl_solver::Solved {
                solution: Solution { placements: vec![Placement { object: owner, x: Some(7.), y: Some(11.), orient: None }], junctions: vec![point], ..Default::default() },
                report: rnl_solver::SolveReport { kept: vec![kept], ..Default::default() },
            },
            author: "test".into(), report_available: true,
        })
    }).expect("the generated pin should remain reachable after materialization");
    assert_eq!(calls.get(), 1);
    let saved = &result.files.iter().find(|f| f.path == "main.sp").unwrap().text;
    assert!(saved.contains("$DOT @ATTACH %mark_R1::p => @PLACE(10, 11)"), "{saved}");
    let read_files = MemoryProvider::new().with("main.sp", saved);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &read_files,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(
        !read.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        read.diagnostics()
    );
    let p = &read.problems[0];
    let mark = p.usages.iter().find(|u| u.definition == "$MARK").unwrap();
    assert_eq!(mark.reference, Reference::Name("%mark_R1".into()));
    let owner = mark.object.unwrap();
    assert!(matches!(
        &p.objects[owner].position,
        Position::Own { x: Slot::Fixed { value: 7., .. }, y: Slot::Fixed { value: 11., .. } }
    ));
    let symbol = &p.symbols[p.objects[owner].symbol.unwrap()];
    assert_eq!(symbol.pins.len(), 1);
    assert_eq!(symbol.pins[0].pos, (1., 0.));
    assert_eq!(symbol.pins[0].dir, Some(0));
    assert!(symbol.pins[0].identity.as_ref().is_some_and(|id| id.names.iter().any(|name| name == "p")));
    let dot = p.usages.iter().find(|u| u.definition == "$DOT").unwrap();
    assert!(matches!(
        &p.objects[dot.object.unwrap()].position,
        Position::Own { x: Slot::Fixed { value: 10., .. }, y: Slot::Fixed { value: 11., .. } }
    ));
}

#[test]
fn coincident_reified_pins_keep_selected_owner_identity() {
    use rnl_runtime::problem::Filler;
    let text = r#"* coincident reified pins
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
R2 c d 1k
* @RNL {
* $MARK = @DEFINE(roles=[["x","instance"]]) {};
* $S = @DEFINE-SYMBOL { %p=(1,0) @PIN(direction="right") };
* #R1 @PLACE(0,0); #R2 @PLACE(3,0);
* # @SELECT-INSTANCE(name="R*") => @AS $MARK;
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
            let mark_for = |label: &str| {
                p.usages
                    .iter()
                    .find(|u| {
                        u.definition == "$MARK"
                            && matches!(&u.roles[0].members[..], [Filler::Object(i)] if p.objects[*i].label == label)
                    })
                    .expect("generated mark for instance")
            };
            let first = mark_for("#R1").object.unwrap();
            let second = mark_for("#R2").object.unwrap();
            assert_ne!(first, second);
            for owner in [first, second] {
                assert!(matches!(p.objects[owner].reference, Reference::Generated { .. }));
                let symbol = &p.symbols[p.objects[owner].symbol.unwrap()];
                assert_eq!(symbol.pins[0].pos, (1., 0.));
            }
            let kept: Vec<_> = p
                .facts
                .iter()
                .enumerate()
                .filter_map(|(i, f)| match &f.kind {
                    FactKind::Place { at: PointRef::Origin(o), .. }
                        if ["#R1", "#R2"].contains(&p.objects[*o].label.as_str()) =>
                    {
                        Some(i)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(kept.len(), 4);
            let placements = [first, second]
                .into_iter()
                .map(|object| Placement { object, x: Some(7.), y: Some(11.), orient: None })
                .collect();
            let target =
                PointRef::SymbolPin { object: first, pin: 0, reference: p.objects[first].reference.then("::p") };
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution { placements, junctions: vec![target], ..Default::default() },
                    report: rnl_solver::SolveReport { kept, ..Default::default() },
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
    .expect("a coincident pin must not replace the chosen pin");
    assert_eq!(calls.get(), 1);
    let saved = &result.files.iter().find(|f| f.path == "main.sp").unwrap().text;
    let saved_files = MemoryProvider::new().with("main.sp", saved);
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &saved_files,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(
        !read.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        read.diagnostics()
    );
    let p = &read.problems[0];
    let owner =
        |name: &str| p.usages.iter().find(|u| u.reference == Reference::Name(name.into())).unwrap().object.unwrap();
    let first = owner("%mark_R1");
    let second = owner("%mark_R2");
    assert_ne!(first, second);
    for i in [first, second] {
        assert!(matches!(
            &p.objects[i].position,
            Position::Own { x: Slot::Fixed { value: 7., .. }, y: Slot::Fixed { value: 11., .. } }
        ));
        assert_eq!(p.symbols[p.objects[i].symbol.unwrap()].pins[0].pos, (1., 0.));
    }
    let dot = p.usages.iter().position(|u| u.definition == "$DOT").unwrap();
    let correct = |point: &PointRef| matches!(point, PointRef::PinPosition { object, pin: 0, reference: Reference::Name(name) } if *object == first && name == "%mark_R1::p::position");
    assert!(p.facts.iter().any(|f| f.chain.contains(&dot)
        && matches!(&f.kind, FactKind::Distance { from, to, .. } if from.iter().chain(to).any(&correct))));
    assert!(p.facts.iter().any(|f| f.chain.contains(&dot)
        && matches!(&f.kind, FactKind::Align { members, .. } if members.iter().any(&correct))));
}
