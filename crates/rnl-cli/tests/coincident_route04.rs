//! Two physically distinct pins at the same point still need a zero-length ROUTE wire.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::Severity,
    solution::{write_record, WirePoint},
    source::MemoryProvider,
    Options,
};
use rnl_solver::{solve, SolverOptions};
use std::{collections::BTreeSet, path::Path};

const SOURCE: &str = r#"* coincident distinct pins
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
R2 a c 1k
* @RNL {
* $S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1,direction="right") };
* {#R1 #R2} @SET-SYMBOL $S => @ORIENT("R0");
* #R1 @PLACE(0,0); #R2 @PLACE(0,0);
* &a @ROUTE-NET
* }
.end
"#;

fn interpret(text: &str) -> rnl_runtime::Interpretation {
    let r = interpret_file(
        Path::new("coincident.sp"),
        None,
        &MemoryProvider::new().with("coincident.sp", text),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

fn exercise(astar_expansions: u64, expect_forced: bool) {
    let r = interpret(SOURCE);
    let p = &r.problems[0];
    assert_eq!(p.routes.len(), 1);
    let original = &p.routes[0].pins;
    assert_eq!(original.len(), 2, "both physical pins must enter the routing tree");
    let keys: BTreeSet<_> = original.iter().map(|p| p.pin_key().expect("electrical pin")).collect();
    assert_eq!(keys.len(), 2, "coordinates must not merge the pin identities");

    // the rows layout's router, whose join through obstacles this is about
    let mut options = SolverOptions::without_time_limit();
    options.settings.layout = rnl_solver::Layout::Rows;
    options.budget.astar_expansions = astar_expansions;
    let solved = solve(p, &options);
    assert!(solved.report.errors.is_empty(), "{:?}", solved.report.errors);
    assert!(solved.report.unrouted.is_empty(), "{:?}", solved.report.unrouted);
    assert_eq!(
        solved.report.notes.iter().any(|n| n.contains("ROUTE trees joined through obstacles")),
        expect_forced,
        "{:?}",
        solved.report.notes
    );
    assert_eq!(solved.solution.wires.len(), 1, "zero-length connection must not disappear");
    let wire = &solved.solution.wires[0];
    assert_eq!(wire.route, 0);
    assert_eq!(wire.points.len(), 2, "no artificial bend or branch is needed: {wire:?}");
    let actual: BTreeSet<_> = wire
        .points
        .iter()
        .map(|p| match p {
            WirePoint::Anchor(anchor) => anchor.pin_key().expect("electrical pin"),
            _ => panic!("both ends must retain physical anchor identities: {wire:?}"),
        })
        .collect();
    assert_eq!(actual, keys);

    let written = write_record(p, &solved.solution, "test", "hash");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    for anchor in original {
        let name = rnl_runtime::problem::point_name(p, anchor);
        assert!(written.record.contains(&name), "missing {name}: {}", written.record);
    }
    let updated = rnl_runtime::writeback::append_record(&r.doc, 0, &r.doc.sources[0].1, &written.record);
    let reread = interpret(&updated);
    assert_eq!(reread.model.levels[0].wires.len(), 1);
    let wire = &reread.model.levels[0].wires[0];
    assert_eq!(wire.points, [(0.0, 0.0), (0.0, 0.0)]);
    assert_eq!(wire.via.len(), 2);
    assert_ne!(wire.via[0], wire.via[1], "writeback must preserve distinct endpoints");
}

#[test]
fn shared_default_keeps_two_coincident_pin_handles() {
    exercise(200_000, false);
}

#[test]
fn exhausted_astar_fallback_keeps_two_coincident_pin_handles() {
    exercise(0, true);
}
