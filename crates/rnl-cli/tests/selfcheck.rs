//! Facts that contradict themselves (standard 8.5, as the draft 0.3 makes it clear for every
//! version, and as 9.5 shows): each fact is checked on its own before the facts of a level are
//! resolved, and one that has no solution on its own fails with an error at its statement, takes
//! no part in the resolution and is not given to a solver. The rest of the statement, and of the
//! usage it is expanded in, stays. ORDER has no solution with two members at one place, DISTANCE
//! none when its sides share a point and min is above 0 or when it measures a point from itself
//! and its bounds leave 0 out, ORIENT-RELATION none with an object on both sides and a relation
//! that turns it; ALIGN and DISTRIBUTE always have one. A file of 0.2 and one of the draft 0.3
//! read the same.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A file of `version` with M1, M2, R1, R2 and R3 and `statements` in one record, the statement
/// `k` on line `line(k)`, at column 5.
fn netlist(version: &str, statements: &[&str]) -> String {
    let mut text = format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n* @RNL {{\n");
    for (k, statement) in statements.iter().enumerate() {
        let end = if k + 1 < statements.len() { ";" } else { "" };
        text += &format!("*   {statement}{end}\n");
    }
    text + "* }\n.model nch nmos level=1\nM1 d1 g1 0 0 nch\nM2 d2 g2 0 0 nch\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\n.end\n"
}

fn line(k: usize) -> u32 {
    4 + k as u32
}

fn read(version: &str, statements: &[&str]) -> Interpretation {
    let text = netlist(version, statements);
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

const VERSIONS: [&str; 2] = ["0.2", "0.3"];

#[test]
fn a_distance_from_an_object_to_itself() {
    for v in VERSIONS {
        let r = read(v, &["%d = #M1 #M1 @DISTANCE(\"x\", min=4)", "%e = #R1 #R1 @DISTANCE(\"left-to-right\", max=-1)"]);
        assert_eq!(
            errors(&r),
            [
                (
                    line(0),
                    fails("%d", "it is a distance from #M1 to itself, which is 0, and min=4 leaves 0 out", "14.5")
                ),
                (
                    line(1),
                    fails("%e", "it is a distance from #R1 to itself, which is 0, and max=-1 leaves 0 out", "14.5")
                ),
            ],
            "{v}"
        );
        assert_eq!(statuses(&r), [("%d".into(), "failed".into()), ("%e".into(), "failed".into())], "{v}");
    }
}

#[test]
fn a_distance_between_sets_that_share_a_member() {
    for v in VERSIONS {
        let r = read(
            v,
            &[
                "%s = {#R1 #R2}",
                "%t = {#R2 #R3}",
                "%d = %s %t @DISTANCE(\"left-to-right\", min=1)",
                "%e = %s %t @DISTANCE(\"x\", exact=2)",
                "%f = %s::position %s @DISTANCE(\"x\", min=1)",
            ],
        );
        let at_most = "so the distance between them is at most 0, below min=";
        let shared = format!("#R2 is on both of its sides, {at_most}");
        let centre =
            format!("%s::position is on one side, and the points it is the centre of on the other, {at_most}1");
        assert_eq!(
            errors(&r),
            [
                (line(2), fails("%d", &format!("{shared}1"), "14.5")),
                (line(3), fails("%e", &format!("{shared}2"), "14.5")),
                (line(4), fails("%f", &centre, "14.5")),
            ],
            "{v}"
        );
    }
}

#[test]
fn an_order_with_two_members_at_one_place() {
    for v in VERSIONS {
        let r = read(
            v,
            &[
                "%o = {#R1 #R2 #R1} @ORDER(\"x\")",
                "%p = {#R2 #R3 #R3::position} @ORDER(\"left-to-right\")",
                "%s = {#R1 #R2}",
                "%q = {#R3 %s::position {%s}} @ORDER(\"x\")",
            ],
        );
        let why = ", and a strict order cannot hold one place twice";
        assert_eq!(
            errors(&r),
            [
                (line(0), fails("%o", &format!("#R1 is a member twice{why}"), "14.3")),
                (line(1), fails("%p", &format!("#R3 and #R3::position are at one place{why}"), "14.3")),
                // the centre of the set, and the set by its centre
                (line(3), fails("%q", &format!("%s::position and a set are at one place{why}"), "14.3")),
            ],
            "{v}"
        );
    }
}

#[test]
fn an_orientation_relation_with_an_object_on_both_sides() {
    for v in VERSIONS {
        let r = read(v, &["%r = #M1 #M1 @ORIENT-RELATION(\"MY\")", "%q = {#M1 #M2} {#M2} @ORIENT-RELATION(\"R90\")"]);
        let why = |m: &str, code: &str| {
            format!("{m} is on both of its sides, and its orientation cannot be its own turned by {code}")
        };
        assert_eq!(
            errors(&r),
            [(line(0), fails("%r", &why("#M1", "MY"), "14.9")), (line(1), fails("%q", &why("#M2", "R90"), "14.9"))],
            "{v}"
        );
    }
}

#[test]
fn what_has_a_solution_on_its_own_takes_part_as_before() {
    for v in VERSIONS {
        // members written twice that ALIGN and DISTRIBUTE take, a shared member with only a max,
        // an exact 0 from an object to itself, a terminal and its instance (14.5), the same set on
        // both sides of R0 (14.9), a set's centre between its members, and its centre a distance
        // from one of them
        let r = read(
            v,
            &[
                "%s = {#R1 #R2}",
                "%s::position #R2 @DISTANCE(\"left-to-right\", min=1)",
                "%s::position %s @DISTANCE(\"x\", max=1)",
                "{#R1 #R1} @ALIGN(\"y\")",
                "{#R1 #R2 #R1} @DISTRIBUTE(\"x\")",
                "{#R1 #R2} {#R2 #R3} @DISTANCE(\"x\", max=2)",
                "#R1 #R1 @DISTANCE(\"x\", exact=0)",
                "#M1:2 #M1 @DISTANCE(\"left-to-right\", min=1)",
                "{#M1 #M2} {#M1 #M2} @ORIENT-RELATION(\"R0\")",
                "{#R1 {#R1 #R2} #R2} @ORDER(\"x\")",
            ],
        );
        assert!(errors(&r).is_empty(), "{v}: {:?}", errors(&r));
        assert!(statuses(&r).iter().all(|(_, s)| s != "failed"), "{v}: {:?}", statuses(&r));
    }
}

/// `$DP_LAYOUT` of 9.5: an ALIGN and a DISTANCE of its two roles.
const DP_LAYOUT: &str = "$DP_LAYOUT = @DEFINE(roles=[[\"in_p\", \"instance\"], [\"in_n\", \"instance\"]]) { %row = {?in_p ?in_n} @ALIGN(\"y\"); %gap = ?in_p ?in_n @DISTANCE(\"x\", min=4) }";

/// The 9.5 example: M1 in both roles of `$DP_LAYOUT`.
const USE: &str = "%dp = #M1(in_p) #M1(in_n) @AS $DP_LAYOUT";

#[test]
fn a_fact_of_a_definition_fails_in_its_body_with_its_application() {
    for v in VERSIONS {
        // the 9.5 example: the expanded DISTANCE asks M1 to be at least 4 from itself
        let r = read(v, &[DP_LAYOUT, USE]);
        let errs: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
        assert_eq!(errs.len(), 1, "{v}: {:?}", errors(&r));
        let e = errs[0];
        // at the DISTANCE in the body, where its statement starts after `%gap =` on the
        // definition's line, with the application as its origin
        let at = e.loc.as_ref().unwrap();
        let col = DP_LAYOUT.find(" ?in_p ?in_n @DISTANCE").unwrap() as u32 + 5;
        assert_eq!((at.line, at.col), (line(0), col), "{v}");
        assert_eq!(
            e.message,
            fails("%gap", "it is a distance from #M1 to itself, which is 0, and min=4 leaves 0 out", "14.5")
        );
        // the application where its statement starts after `%dp =`
        let col = USE.find(" #M1").unwrap() + 5;
        assert_eq!(e.origin, format!("applying $DP_LAYOUT (t.sp:{}:{col})", line(1)));
        // only it: the ALIGN of the same usage stays
        let mut st = statuses(&r);
        st.sort();
        assert_eq!(st, [("%gap".into(), "failed".into()), ("%row".into(), "unsolved".into())], "{v}");
    }
}

#[test]
fn a_fact_without_a_solution_is_not_given_to_a_solver() {
    for v in VERSIONS {
        let text = netlist(v, &[DP_LAYOUT, USE, "%o = {#R1 #R2 #R1} @ORDER(\"x\")"]);
        let files = MemoryProvider::new().with("t.sp", &text);
        let solver = Options { std: false, ..Options::solver() };
        let r = interpret_file(Path::new("t.sp"), None, &files, &solver).unwrap();
        let p: &LevelProblem = &r.problems[0];
        let labels: Vec<&str> = p.facts.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, ["%row"], "{v}");
    }
}
