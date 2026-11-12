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

fn numbered_roles(version: &str) -> String {
    format!(
        r#"* numbered roles
* @RNL(type="header") {{ @VERSION("{version}") }}
R1 a b 1k
* @RNL {{
* $ANCHOR = @DEFINE(roles=[]) {{}};
* $ANCHOR @ATTACH;
* $GROUPING = @DEFINE(roles=["{{members}}"] ) {{ (1,2) @TEXT("body") }};
* # @SELECT-USAGE $ANCHOR => @AS $GROUPING
* }}
.end
"#
    )
}

#[test]
fn authored_numbered_role_members_survive_generated_writeback() {
    use rnl_runtime::solution::{write_record, Placement, Solution};
    let text = numbered_roles("0.4");
    let files = MemoryProvider::new().with("main.sp", &text);
    let opts = Options { std: false, generated: false, problems: true, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    assert!(original.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", original.diagnostics());
    let p = &original.problems[0];
    let usage = p.usages.iter().find(|u| u.rule_made).unwrap();
    assert_eq!(usage.roles[0].written.as_deref(), Some("{%#1}"));
    assert!(usage.written_numbers > 0, "the generated body must contribute to the numbering check");
    let object = usage.object.unwrap();
    let solution = Solution {
        placements: vec![Placement { object, x: Some(3.), y: Some(5.), orient: None }],
        ..Default::default()
    };
    let written = write_record(p, &solution, "test", "test");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    let record = rnl_view::finish_record(&original.doc, &files, &opts, p, &written.record);
    assert!(record.contains("numbering=\""), "{record}");
    let result = rnl_runtime::writeback::replace_generated_with_header(&original.doc, p.level, &text, &record, "0.4");
    let back = interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &result),
        &Options { generated: true, ..opts },
    )
    .unwrap();
    assert!(back.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", back.diagnostics());
    assert_eq!(drawn(p, &back)[object].at, [Some(3.), Some(5.)]);
    let reified = back.problems[0].usages.iter().find(|u| u.definition == "$GROUPING" && !u.rule_made).unwrap();
    assert_eq!(reified.roles[0].members, usage.roles[0].members);
}

#[test]
fn released_numbered_role_export_is_unchanged() {
    let files = MemoryProvider::new().with("main.sp", &numbered_roles("0.3"));
    let read =
        interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let usage = read.problems[0].usages.iter().find(|u| u.definition == "$GROUPING").unwrap();
    assert_eq!(usage.roles[0].written, None);
}

#[test]
fn numbers_owned_by_the_replaced_record_are_not_exported_as_role_references() {
    let text = numbered_roles("0.4")
        .replace("* $ANCHOR @ATTACH;\n", "")
        .replace("* # @SELECT-USAGE $ANCHOR => @AS $GROUPING\n", "");
    let files = MemoryProvider::new().with("main.sp", &text);
    let opts = Options { std: false, generated: false, problems: true, ..Default::default() };
    let original = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let p = &original.problems[0];
    assert_eq!(p.authored_numbers, 0);
    let record = rnl_view::finish_record(
        &original.doc,
        &files,
        &opts,
        p,
        "@RNL(type=\"generated\",author=\"test\") { $ANCHOR @ATTACH; $GROUPING @ATTACH {%#1} }",
    );
    let text = rnl_runtime::writeback::replace_generated_with_header(&original.doc, p.level, &text, &record, "0.4");
    let read = interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { generated: true, ..opts },
    )
    .unwrap();
    assert!(read.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", read.diagnostics());
    let usage = read.problems[0].usages.iter().find(|u| u.definition == "$GROUPING").unwrap();
    assert_eq!(usage.roles[0].written, None, "a new record cannot reuse an old record's numbered usage");
}
