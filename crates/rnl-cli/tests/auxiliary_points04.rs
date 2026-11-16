//! The point of a usage that only helps the solve (10.5, 14.8): made by a rule, not written out,
//! drawing nothing. The record does not write it and does not count it unsolved; a usage that a
//! rule makes, that is not written out and that has a symbol still counts.
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_cli::solve::{solve, SolveOptions};
use rnl_runtime::diag::codes;
use rnl_runtime::problem::{LevelProblem, PointRef, Position};
use rnl_runtime::solution::{auxiliary_points, write_record, Placement, Solution};
use rnl_runtime::source::{Layered, MemoryProvider};
use rnl_runtime::Options;
use rnl_solver::SolverOptions;

/// The body of the column: its point and the instances in one column.
const COLUMN: &str = "{ {?xs ?} @ALIGN(\"x\", priority=\"low\") }";

/// A level whose rule makes one usage of a private column over its two instances, at least 6 units
/// left of R1, the column's definition having `body`; `extra` goes after it.
fn level(body: &str, extra: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\nR2 c d 2k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (4,0) @PIN(terminal=2) }}; {{#R1 #R2}} @SET-SYMBOL $S; #R1 @PLACE(0,0); #R2 @PLACE(10,0); %col = @DEFINE(roles=[\"{{xs}}\"]) {body}; # @SELECT-INSTANCE => %col @ATTACH => %cols; %cols #R1 @DISTANCE(\"left-to-right\", min=6, priority=\"low\"); {extra} }}\n.end\n"
    )
}

/// The unsolved decisions of the record when the solver places the column's point, with whether
/// that point only helps the solve.
fn record_of(extra: &str) -> (bool, Vec<rnl_runtime::diag::Code>) {
    record_with(COLUMN, extra, |_, _| {})
}

/// [`record_of`] with a column of definition body `body`, the problem changed by `change`, which
/// is given the index of the column's point.
fn record_with(
    body: &str,
    extra: &str,
    change: impl Fn(&mut LevelProblem, usize),
) -> (bool, Vec<rnl_runtime::diag::Code>) {
    let text = level(body, extra);
    let files = MemoryProvider::new().with("t.sp", &text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let r = interpret_file(Path::new("t.sp"), None, &files, &opts).unwrap();
    let mut p = r.problems[0].clone();
    let usage = p
        .usages
        .iter()
        .find(|u| u.definition.ends_with("col"))
        .unwrap_or_else(|| panic!("{:?}", p.usages.iter().map(|u| &u.definition).collect::<Vec<_>>()));
    let point = usage.object.expect("the column has its own point");
    change(&mut p, point);
    let p = &p;
    let solution = Solution {
        placements: vec![Placement { object: point, x: Some(-6.0), y: Some(0.0), orient: None }],
        ..Default::default()
    };
    let written = write_record(p, &solution, "test", "test");
    (auxiliary_points(p)[point], written.unsolved.iter().map(|u| u.code).collect())
}

#[test]
fn the_point_of_a_column_that_draws_nothing_is_not_written_and_not_unsolved() {
    let (auxiliary, unsolved) = record_of("");
    assert!(auxiliary);
    assert!(unsolved.is_empty(), "{unsolved:?}");
}

#[test]
fn a_column_with_a_symbol_is_still_unsolved_when_it_cannot_be_written_out() {
    let (auxiliary, unsolved) =
        record_of("$MARK = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; %cols @SET-SYMBOL $MARK");
    assert!(!auxiliary);
    assert_eq!(unsolved, vec![codes::UNSOLVED_NOT_WRITTEN_OUT]);
}

#[test]
fn a_column_that_another_object_is_drawn_at_is_still_unsolved_when_it_cannot_be_written_out() {
    // R2 drawn at the column's point (12.1): the point places something, so it is written or unsolved
    let (auxiliary, unsolved) = record_with(COLUMN, "", |p, point| {
        p.objects[1].position = Position::At(PointRef::Origin(point));
    });
    assert!(!auxiliary);
    assert_eq!(unsolved, vec![codes::UNSOLVED_NOT_WRITTEN_OUT]);
}

/// A subcircuit with two ports, a net of three pins routed with a junction, and the fill of the
/// standard package, which puts the port tags in a column.
const FILLED: &str = "* fill\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n.subckt cell in out\nR1 in mid 1k\nR2 mid out 1k\nR3 mid 0 1k\n* @RNL { &mid @ROUTE-NET(priority=\"high\") }\n.ends cell\n.end\n";

#[test]
fn a_level_with_a_port_column_and_a_junction_is_solved_and_solved_again_the_same() {
    let files = MemoryProvider::new().with("fill.sp", FILLED);
    // no safety limit of wall-clock time either: a loaded machine gives the same drawing
    let solver = SolverOptions { safety_seconds: None, ..SolverOptions::without_time_limit() };
    let options = SolveOptions { solver, ..SolveOptions::default() };
    let a = solve(Path::new("fill.sp"), &files, &options).unwrap();
    let text = &a.files[0].text;
    assert!(text.contains("$IO_PORT @ATTACH &in"), "{text}");
    assert!(!text.contains("port_column"), "{text}");
    assert!(text.contains("$JUNCTION @ATTACH"), "{text}");
    assert_eq!(a.errors, 0);
    for level in &a.levels {
        assert!(level.written.unsolved.is_empty(), "{:?}", level.written.unsolved);
    }
    // the solver reads the file without its records of the solver (2.8): the record replaces itself
    let again = MemoryProvider::new().with(&a.files[0].path, text);
    let b = solve(Path::new("fill.sp"), &Layered { layers: vec![&again, &files] }, &options).unwrap();
    assert_eq!(b.files, a.files);
}
