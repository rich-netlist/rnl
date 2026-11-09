//! ASSERT-TOPOLOGY (15.4): that some members are connected in series, in parallel, as a star or as
//! a single loop. A check that does not hold is a violation (7.6); nothing fails.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version`: a chain R1–R3, two resistors R4 and R5 on one net, a T network L1, C1,
/// L2, a pi network C3, L3, C4, a pair M3 and M4, two branches R6–R7 and R8 between p and q, and
/// `body` as the last record.
fn netlist(version: &str, body: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n.model nch nmos level=1\n\
         R1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 x y 1k\nR5 x z 1k\n\
         L1 t1 n 1n\nC1 n t2 1p\nL2 n t3 1n\n\
         C3 u v 1p\nL3 v w 1n\nC4 w u 1p\n\
         M3 d1 g1 s1 0 nch\nM4 d1 g2 s1 0 nch\n\
         R6 p m 1k\nR7 m q 1k\nR8 p q 1k\n\
         * @RNL {{ {body} }}\n.end\n"
    )
}

fn read(text: &str) -> Interpretation {
    let options = Options { std: false, ..Options::default() };
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), &options).unwrap()
}

fn of(r: &Interpretation, severity: Severity) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| (d.code, r.sink.render(d))).collect()
}

fn violations(body: &str) -> Vec<(Code, String)> {
    let r = read(&netlist("0.4", body));
    assert_eq!(of(&r, Severity::Error), Vec::new(), "{body}");
    of(&r, Severity::Violation)
}

/// The one violation of `body`; its text.
fn violation(body: &str) -> String {
    let v = violations(body);
    assert_eq!(v.len(), 1, "{body}: {v:?}");
    assert_eq!(v[0].0, codes::VIOLATION_TOPOLOGY, "{body}: {v:?}");
    v[0].1.clone()
}

fn error(body: &str, code: Code) -> String {
    let r = read(&netlist("0.4", body));
    let e = of(&r, Severity::Error);
    assert_eq!(e.len(), 1, "{body}: {e:?}");
    assert_eq!(e[0].0, code, "{body}: {e:?}");
    e[0].1.clone()
}

#[test]
fn the_four_topologies_that_hold_give_nothing() {
    for body in [
        "{#R1 #R2 #R3} @ASSERT-TOPOLOGY(\"series\")",
        "{#M3 #M4} @ASSERT-TOPOLOGY(\"parallel\")",
        "{#L1 #C1 #L2} @ASSERT-TOPOLOGY(\"star\")",
        "{#C3 #L3 #C4} @ASSERT-TOPOLOGY(\"loop\")",
        // a set given alone is split one level: its items are the members
        "%chain = {#R1 #R2 #R3}; %chain @ASSERT-TOPOLOGY(\"series\")",
        // each branch a black box, whose inner net m is used by nothing outside it
        "%b1 = {#R6 #R7}; %b2 = {#R8}; {%b1 %b2} @ASSERT-TOPOLOGY(\"parallel\")",
        // with fewer than two members it holds trivially
        "{#R1} @ASSERT-TOPOLOGY(\"series\")",
    ] {
        assert_eq!(violations(body), Vec::new(), "{body}");
    }
}

#[test]
fn members_not_connected_as_said_are_a_violation() {
    let v = violation("{#R1 #R3 #R2} @ASSERT-TOPOLOGY(\"series\", message=\"the divider\")");
    assert!(v.starts_with("violation t.sp:"), "{v}");
    assert!(
        v.contains(": RNL6005 violation-topology: @ASSERT-TOPOLOGY does not hold: the members should be in series"),
        "{v}"
    );
    assert!(v.contains("#R1 and #R3 share no net") && v.contains("; the divider (15.4)"), "{v}");
    let v = violation("{#R4 #R5} @ASSERT-TOPOLOGY(\"parallel\")");
    assert!(v.contains("sharing at least two nets, the ends, but the nets all of them are on are &x"), "{v}");
    let v = violation("{#R1 #R2 #R3} @ASSERT-TOPOLOGY(\"star\")");
    assert!(v.contains("the members should be a star"), "{v}");
    let v = violation("{#R1 #R2 #R3} @ASSERT-TOPOLOGY(\"loop\")");
    assert!(v.contains("#R1 and #R3 share no net"), "{v}");
    let v = violation("{#R6 #R7} @ASSERT-TOPOLOGY(\"loop\")");
    assert!(
        v.contains("a loop has at least three members, and #R6 and #R7 are two; for two members write \"parallel\""),
        "{v}"
    );
}

#[test]
fn exclude_takes_out_terminals_of_a_type() {
    // the pair shares d1, s1 and 0: in series only with the drains and bulks taken out
    assert_eq!(violations("{#M3 #M4} @ASSERT-TOPOLOGY(\"series\")").len(), 1);
    for exclude in ["[[\"nmos\", \"d\"], [\"NMOS\", \"b\"]]", "[[\"nmos\", 1], [\"nmos\", 4]]"] {
        let body = format!("{{#M3 #M4}} @ASSERT-TOPOLOGY(\"series\", exclude={exclude})");
        assert_eq!(violations(&body), Vec::new(), "{body}");
    }
    // a type the list does not name, or a terminal the instance does not have, takes out nothing
    assert_eq!(
        violations("{#M3 #M4} @ASSERT-TOPOLOGY(\"series\", exclude=[[\"pmos\", \"d\"], [\"nmos\", \"x\"]])").len(),
        1
    );
}

#[test]
fn wrong_parameters_and_members_are_errors() {
    for exclude in ["[\"nmos\", \"g\"]", "[[\"nmos\"]]", "[[\"nmos\", 1.5]]", "[[\"\", \"g\"]]"] {
        let body = format!("{{#M3 #M4}} @ASSERT-TOPOLOGY(\"parallel\", exclude={exclude})");
        let e = error(&body, codes::ASSERT_TOPOLOGY_EXCLUDE_NOT_PAIR);
        assert!(e.contains("exclude= of @ASSERT-TOPOLOGY is a list of pairs of a type and a terminal"), "{e}");
    }
    let e = error("{#R1 &a} @ASSERT-TOPOLOGY(\"series\")", codes::ASSERT_TOPOLOGY_MEMBER_WRONG_KIND);
    assert!(e.contains("takes instances, bundles, usages and sets, and &a is not"), "{e}");
    let r = read(&netlist("0.4", "{#R1 #R2} @ASSERT-TOPOLOGY(\"ring\")"));
    assert_eq!(of(&r, Severity::Error).len(), 1, "{:?}", r.diagnostics());
}

#[test]
fn in_a_definition_it_is_checked_for_each_application() {
    let def = "$CHAIN = @DEFINE(roles=[[\"first\", \"instance\"], [\"second\", \"instance\"]]) { {?first ?second} @ASSERT-TOPOLOGY(\"series\") };";
    assert_eq!(violations(&format!("{def} #R1(first) #R2(second) @AS $CHAIN")), Vec::new());
    let v = violation(&format!("{def} %c = #R1(first) #R3(second) @AS $CHAIN"));
    assert!(v.contains("@ASSERT-TOPOLOGY does not hold in %c, a usage of $CHAIN at t.sp:"), "{v}");
    assert_eq!(violations(&format!("{def} #R1(first) #R3(second) @AS(priority=\"disabled\") $CHAIN")), Vec::new());
}

#[test]
fn files_of_0_3_have_no_topology_assertion() {
    let r = read(&netlist("0.3", "{#R1 #R2} @ASSERT-TOPOLOGY(\"series\")"));
    let e = of(&r, Severity::Error);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(
        e[0].1.contains("unknown keyword @ASSERT-TOPOLOGY: the language 0.4 adds it, and this file is of 0.3"),
        "{e:?}"
    );
}
