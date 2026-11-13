//! Strict inputs (7.3, 5.5): in a file of 0.4 a failed object written in a set given to a
//! constraint or to AS makes the whole statement fail; it does not become a statement about the
//! other members. A statement nested in the set that does not depend on the failure still takes
//! effect. A file of 0.3 drops the failed member, as it did.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::problem::{FactKind, LevelProblem};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version` with R1 to R4 and M1, M2, whose last record writes `body`; there is no R9.
fn read(version: &str, body: &str) -> Interpretation {
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d 0 1k\n\
         .model nch nmos level=1\nM1 a b c 0 nch\nM2 b c d 0 nch\n* @RNL {{ {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

fn problem(r: &Interpretation) -> &LevelProblem {
    &r.problems[0]
}

/// Whether the problem has a fact of the keyword `kw`.
fn has_fact(p: &LevelProblem, kw: &str) -> bool {
    p.facts.iter().any(|f| f.kind.name() == kw)
}

#[test]
fn a_failed_member_of_a_set_fails_the_whole_constraint() {
    for (body, kw) in [
        ("{#R1 #R2 #R9} @ALIGN(\"y\")", "ALIGN"),
        ("{#R1 #R2 #R9} @ORDER(\"left-to-right\")", "ORDER"),
        ("{#R1 #R2 #R3 #R9} @DISTRIBUTE(\"left-to-right\")", "DISTRIBUTE"),
        ("{#R1 #R9} #R2 @DISTANCE(\"left-to-right\", min=1)", "DISTANCE"),
        ("#M1 {#M2 #M9} @ORIENT-RELATION(\"MY\")", "ORIENT-RELATION"),
        // a set written deeper, or named, is the same
        ("{#R1 {#R2 #R9}} @ORDER(\"left-to-right\")", "ORDER"),
        ("%s = {#R1 #R9}; %s @ALIGN(\"y\")", "ALIGN"),
    ] {
        let r = read("0.4", body);
        let e = errors(&r);
        assert!(e.contains(&codes::REF_NO_INSTANCE), "{body}: {e:?}");
        assert!(e.contains(&codes::CONSTRAINT_MEMBER_FAILED), "{body}: {e:?}");
        // nothing of it is given to a solver
        assert!(!has_fact(problem(&r), kw), "{body}: {:?}", problem(&r).facts);
    }
}

#[test]
fn a_statement_nested_in_the_set_still_takes_effect() {
    // 5.5: the ALIGN fails, the coordinates of R1 still take effect
    let r = read("0.4", "{%a = #R1 @PLACE(1, 2); #R9} @ALIGN(\"y\")");
    assert!(errors(&r).contains(&codes::CONSTRAINT_MEMBER_FAILED));
    let p = problem(&r);
    assert!(!has_fact(p, "ALIGN"));
    assert!(p.facts.iter().any(|f| matches!(f.kind, FactKind::Place { .. })), "{:?}", p.facts);
}

#[test]
fn a_failed_member_of_a_set_role_fails_the_application() {
    let def = "$PAIR = @DEFINE(roles=[\"{s}\"]) { {?s} @ALIGN(\"y\") };";
    let r = read("0.4", &format!("{def} %u = {{#R1 #R9}}(s) @AS $PAIR"));
    assert!(errors(&r).contains(&codes::USAGE_ROLE_FAILED_OBJECT), "{:?}", errors(&r));
    assert!(!has_fact(problem(&r), "ALIGN"));
    // with every member found, the usage is made
    let r = read("0.4", &format!("{def} %u = {{#R1 #R2}}(s) @AS $PAIR"));
    assert_eq!(errors(&r), Vec::new());
    assert!(has_fact(problem(&r), "ALIGN"));
}

#[test]
fn a_set_whose_members_are_all_found_is_unchanged() {
    let r = read("0.4", "{#R1 #R2 #R3} @ALIGN(\"y\"); # @SELECT-INSTANCE(name=\"R*\") => @ORDER(\"left-to-right\")");
    assert_eq!(errors(&r), Vec::new());
    assert!(has_fact(problem(&r), "ALIGN") && has_fact(problem(&r), "ORDER"));
}

#[test]
fn a_file_of_0_3_drops_the_failed_member_as_it_did() {
    let r = read("0.3", "{#R1 #R2 #R9} @ALIGN(\"y\")");
    let e = errors(&r);
    assert_eq!(e, vec![codes::REF_NO_INSTANCE]);
    assert!(has_fact(problem(&r), "ALIGN"));
}
