//! Facts that contradict themselves (8.5): each fact is checked on its own before the facts of a
//! level are resolved, and one that has no solution on its own fails with an error at its
//! statement, takes no part in the resolution and is not given to a solver. The rest of the
//! statement, and of the usage it is expanded in, stays. ORIENT-RELATION has none with an object on
//! both sides and a relation that turns it. An ORDER or a DISTANCE stands for one constraint for
//! each combination of its members, and a combination that takes one object twice is left out
//! (14.1): one that is left with no two different objects holds trivially, with a hint, and fails
//! nothing; ALIGN and DISTRIBUTE always have a solution.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A file with M1, M2, R1, R2 and R3 and `statements` in one record, the statement `k` on line
/// `line(k)`, at column 5.
fn netlist(statements: &[&str]) -> String {
    let mut text = String::from("* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n* @RNL {\n");
    for (k, statement) in statements.iter().enumerate() {
        let end = if k + 1 < statements.len() { ";" } else { "" };
        text += &format!("*   {statement}{end}\n");
    }
    text + "* }\n.model nch nmos level=1\nM1 d1 g1 0 0 nch\nM2 d2 g2 0 0 nch\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\n.end\n"
}

fn line(k: usize) -> u32 {
    4 + k as u32
}

fn read(statements: &[&str]) -> Interpretation {
    let text = netlist(statements);
    interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &text),
        &Options { std: false, ..Options::default() },
    )
    .unwrap()
}

/// The errors, by their line and message.
fn errors(r: &Interpretation) -> Vec<(u32, String)> {
    let found = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error);
    found.map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone())).collect()
}

/// The statuses of the constraints of the main level, by label.
fn statuses(r: &Interpretation) -> Vec<(String, String)> {
    r.model.levels[0].constraints.iter().map(|c| (c.label.clone(), c.status.clone())).collect()
}

fn fails(what: &str, why: &str, section: &str) -> String {
    format!("{what} has no solution on its own: {why}; this constraint fails (8.5, {section})")
}

/// The hints, by their line and message.
fn hints(r: &Interpretation) -> Vec<(u32, String)> {
    let found = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Hint && d.loc.is_some());
    found.map(|d| (d.loc.as_ref().unwrap().line, d.message.clone())).collect()
}

/// The hint of a constraint `kind` all of whose combinations take one object twice (14.1).
fn trivial(kind: &str) -> String {
    format!(
        "{kind} relates no two different objects once a combination that takes one object twice is left out, so \
         its constraints hold trivially (14.1)"
    )
}

#[test]
fn a_distance_or_an_order_that_takes_one_object_twice_holds_trivially() {
    let r = read(&[
        "%d = #M1 #M1 @DISTANCE(\"left-to-right\", min=4)",
        "%o = {#R1 #R2 #R1} @ORDER(\"left-to-right\")",
        "%p = {#R2 #R3 #R3::position} @ORDER(\"left-to-right\")",
    ]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(
        hints(&r).into_iter().filter(|(_, m)| m.contains("(14.1)")).collect::<Vec<_>>(),
        [(line(0), trivial("DISTANCE")), (line(1), trivial("ORDER")), (line(2), trivial("ORDER"))]
    );
    assert!(statuses(&r).iter().all(|(_, s)| s != "failed"), "{:?}", statuses(&r));
}

#[test]
fn a_distance_between_sets_that_share_a_member() {
    // one constraint for each pair (14.5), the pair of the member they share left out (14.1)
    let r = read(&["%s = {#R1 #R2}", "%t = {#R2 #R3}", "%d = %s %t @DISTANCE(\"left-to-right\", min=1)"]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(statuses(&r), vec![("%d".to_string(), "unsolved".to_string()); 3]);
}

#[test]
fn an_orientation_relation_with_an_object_on_both_sides() {
    let r = read(&["%r = #M1 #M1 @ORIENT-RELATION(\"MY\")", "%q = {#M1 #M2} {#M2} @ORIENT-RELATION(\"R90\")"]);
    let why = |m: &str, code: &str| {
        format!("{m} is on both of its sides, and its orientation cannot be its own turned by {code}")
    };
    assert_eq!(
        errors(&r),
        [(line(0), fails("%r", &why("#M1", "MY"), "14.9")), (line(1), fails("%q", &why("#M2", "R90"), "14.9"))],
    );
}

#[test]
fn what_has_a_solution_on_its_own_takes_part_as_before() {
    // members written twice that ALIGN and DISTRIBUTE take, a set on both sides of R0 (14.9), a
    // terminal and its instance (14.5), and a set within the members of an ORDER (14.1)
    let r = read(&[
        "%s = {#R1 #R2}",
        "%s::position #R2 @DISTANCE(\"left-to-right\", min=1)",
        "{#R1 #R1} @ALIGN(\"y\")",
        "{#R1 #R2 #R1} @DISTRIBUTE(\"left-to-right\")",
        "{#R1 #R2} {#R2 #R3} @DISTANCE(\"left-to-right\", max=2)",
        "#M1:2 #M1 @DISTANCE(\"left-to-right\", min=1)",
        "{#M1 #M2} #M1 @ORIENT-RELATION(\"R0\")",
        "{#R1 {#R2 #R3} #M1} @ORDER(\"left-to-right\")",
    ]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(statuses(&r).iter().all(|(_, s)| s != "failed"), "{:?}", statuses(&r));
}

/// `$MIRRORED`: an ALIGN of the roles `left` and `third`, and an ORIENT-RELATION that mirrors
/// `left` into `right`.
const MIRRORED: &str = "$MIRRORED = @DEFINE(roles=[[\"left\", \"instance\"], [\"right\", \"instance\"], [\"third\", \"instance\"]]) { %row = {?left ?third} @ALIGN(\"y\"); %turn = ?left ?right @ORIENT-RELATION(\"MY\") }";

/// M1 in both roles of `$MIRRORED` that the ORIENT-RELATION relates.
const USE: &str = "%m = #M1(left) #M1(right) #M2(third) @AS $MIRRORED";

#[test]
fn a_fact_of_a_definition_fails_in_its_body_with_its_application() {
    // the expanded ORIENT-RELATION asks M1 to be its own mirror image
    let r = read(&[MIRRORED, USE]);
    let errs: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(errs.len(), 1, "{:?}", errors(&r));
    let e = errs[0];
    // at the ORIENT-RELATION in the body, where its statement starts after `%turn =` on the
    // definition's line, with the application as its origin
    let at = e.loc.as_ref().unwrap();
    let col = MIRRORED.find(" ?left ?right @ORIENT-RELATION").unwrap() as u32 + 5;
    assert_eq!((at.line, at.col), (line(0), col));
    let why = "#M1 is on both of its sides, and its orientation cannot be its own turned by MY";
    assert_eq!(e.message, fails("%turn", why, "14.9"));
    // the application where its statement starts after `%m =`
    let col = USE.find(" #M1").unwrap() + 5;
    assert_eq!(e.origin, format!("applying $MIRRORED (t.sp:{}:{col})", line(1)));
    // only it: the ALIGN of the same usage stays
    let mut st = statuses(&r);
    st.sort();
    assert_eq!(st, [("%row".into(), "unsolved".into()), ("%turn".into(), "failed".into())]);
}

#[test]
fn a_fact_without_a_solution_is_not_given_to_a_solver() {
    let text = netlist(&[MIRRORED, USE, "%r = #M2 #M2 @ORIENT-RELATION(\"MY\")"]);
    let files = MemoryProvider::new().with("t.sp", &text);
    let solver = Options { std: false, ..Options::solver() };
    let r = interpret_file(Path::new("t.sp"), None, &files, &solver).unwrap();
    let p: &LevelProblem = &r.problems[0];
    let labels: Vec<&str> = p.facts.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(labels, ["%row"]);
}
