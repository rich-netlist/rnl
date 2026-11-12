//! Selected symbol pins must retain writable identities when a rule fills a usage role.
use std::path::Path;

use rnl_cli::{
    interpret_file,
    solve::{solve, SolveOptions},
};
use rnl_runtime::{diag::Severity, previous::drawn, source::MemoryProvider, Options};
use rnl_solver::{problem_fact_holds_as_drawn, SolverOptions};

fn source(pins: &str) -> String {
    format!(
        r#"* generated pin roles
* @RNL(type="header") {{ @VERSION("0.4") }}
R1 a b 1k
* @RNL {{
* $S = @DEFINE-SYMBOL {{ (-1,-1) (1,1) @RECTANGLE; {pins} }};
* $D = @DEFINE(roles=[["p","pin"]]) {{ %q=(_,_); {{?p %q}} @ALIGN("x"); ?p %q @DISTANCE("bottom-to-top",min=2); ?p::net @WIRE {{?p %q}} }};
* #R1 @SET-SYMBOL $S => @PLACE(0,0);
* #R1 @SELECT-PIN(wired=false) => @AS $D
* }}
.end
"#
    )
}

#[test]
fn selected_pin_roles_write_out_and_keep_their_facts_after_reopening() {
    for (pins, references) in [
        ("%left=(-2,0) @PIN(terminal=1); %right=(2,0) @PIN(terminal=2)", ["#R1::left", "#R1::right"]),
        ("(-2,0) @PIN(terminal=1); (2,0) @PIN(terminal=2)", ["#R1:p", "#R1:n"]),
        ("%\"left pin\"=(-2,0) @PIN(terminal=1); %right=(2,0) @PIN(terminal=2)", ["#R1::\"left pin\"", "#R1::right"]),
    ] {
        let files = MemoryProvider::new().with("pins.sp", &source(pins));
        let options = SolveOptions { std: false, solver: SolverOptions::without_time_limit(), ..Default::default() };
        let run = solve(Path::new("pins.sp"), &files, &options).unwrap();
        assert_eq!(run.errors, 0);
        let level = &run.levels[0];
        assert_eq!(level.solved.report.kept.len(), level.problem.facts.len());
        let roles: Vec<_> =
            level.problem.usages.iter().filter(|u| u.rule_made).map(|u| u.roles[0].written.as_deref()).collect();
        assert_eq!(roles, references.map(Some));
        assert!(level.written.unsolved.is_empty(), "{:?}", level.written.unsolved);
        let written = MemoryProvider::new().with("pins.sp", &run.files[0].text);
        let back = interpret_file(
            Path::new("pins.sp"),
            None,
            &written,
            &Options { std: false, generated: true, problems: true, ..Options::default() },
        )
        .unwrap();
        assert!(back.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", back.diagnostics());
        assert_eq!(back.problems[0].usages.len(), 2);
        let coordinates = drawn(&level.problem, &back);
        for &fact in &level.solved.report.kept {
            assert_eq!(
                problem_fact_holds_as_drawn(&level.problem, &coordinates, fact, 1e-6),
                Some(true),
                "fact {fact}, {pins}"
            );
        }
    }
}

#[test]
fn an_unnamed_shadowed_pin_cannot_borrow_a_later_pins_terminal() {
    let text = source("(-2,0) @PIN(terminal=1); %last=(2,0) @PIN(terminal=1)");
    let files = MemoryProvider::new().with("pins.sp", &text);
    let read =
        interpret_file(Path::new("pins.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(read.diagnostics().iter().all(|d| d.severity != Severity::Error));
    let roles: Vec<_> =
        read.problems[0].usages.iter().filter(|u| u.rule_made).map(|u| u.roles[0].written.as_deref()).collect();
    assert_eq!(roles, [None, Some("#R1::last")]);
}
