//! Assertions (15.3): ASSERT-INSTANCE, ASSERT-TERMINAL and ASSERT-NET check that what a file says
//! agrees with the circuit, and a check that does not hold is a violation (7.6). Nothing fails.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version`: a pair of transistors M1 and M2 and a dummy M3, a resistor R1 and the
/// subcircuit `cell`, whose port is p; `body` is the last record.
fn netlist(version: &str, body: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n.model nch nmos level=1\n.model pch pmos level=1\n\
         M1 d1 g s 0 nch w=1u l=1u\nM2 d2 g s 0 nch w=1u l=2u\nM3 s s s 0 pch w=1u\nR1 d1 vdd 1k\nX1 vdd cell\n\
         .subckt cell p\nR2 p 0 1k\n.ends cell\n* @RNL {{ {body} }}\n.end\n"
    )
}

fn read(text: &str) -> Interpretation {
    let options = Options { std: false, ..Options::default() };
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), &options).unwrap()
}

/// The diagnostics of a severity, as `rnl check` prints them, with their kinds.
fn of(r: &Interpretation, severity: Severity) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| (d.code, r.sink.render(d))).collect()
}

fn violations(body: &str) -> Vec<(Code, String)> {
    let r = read(&netlist("0.4", body));
    assert_eq!(of(&r, Severity::Error), Vec::new(), "{body}");
    of(&r, Severity::Violation)
}

/// The one violation of `body`, which is of the kind `code`; its text.
fn violation(body: &str, code: Code) -> String {
    let v = violations(body);
    assert_eq!(v.len(), 1, "{body}: {v:?}");
    assert_eq!(v[0].0, code, "{body}: {v:?}");
    v[0].1.clone()
}

fn error(body: &str, code: Code) -> String {
    let r = read(&netlist("0.4", body));
    let e = of(&r, Severity::Error);
    assert_eq!(e.len(), 1, "{body}: {e:?}");
    assert_eq!(e[0].0, code, "{body}: {e:?}");
    assert_eq!(of(&r, Severity::Violation), Vec::new(), "{body}");
    e[0].1.clone()
}

#[test]
fn an_assertion_that_holds_gives_nothing() {
    for body in [
        "{#M1 #M2} @ASSERT-INSTANCE(same=[\"model\", \"w\"], different=[\"name\"], has=[\"l\"], type=\"nmos\")",
        "{#M1 #M2} @ASSERT-INSTANCE(model=\"nch\", name=\"M*\", terminals=4)",
        // no member writes nf: absent is a value of its own, the same for all (15.3)
        "{#M1 #M2} @ASSERT-INSTANCE(same=[\"nf\"])",
        "{#M1:3 #M2:3} @ASSERT-TERMINAL(same=[\"net\"], net=\"s\", terminal=3)",
        "{&d1 &d2} @ASSERT-NET(different=[\"name\"], port=false, global=false)",
        // a set with no members holds trivially
        "# @SELECT-INSTANCE(name=\"Q*\") => @ASSERT-INSTANCE(type=\"npn\")",
    ] {
        assert_eq!(violations(body), Vec::new(), "{body}");
    }
}

#[test]
fn a_condition_that_a_member_does_not_meet_is_a_violation() {
    let v = violation(
        "{#M1 #M3} @ASSERT-INSTANCE(type=\"nmos\", message=\"the pair is of nmos\")",
        codes::VIOLATION_CONDITION,
    );
    assert!(v.starts_with("violation t.sp:"), "{v}");
    assert!(
        v.contains(": RNL6000 violation-condition: @ASSERT-INSTANCE does not hold: every member should have type \"nmos\", but #M3 has type \"pmos\"; the pair is of nmos (15.3)"),
        "{v}"
    );
    let v = violation("{#M1:1 #M2:1} @ASSERT-TERMINAL(net=\"d1\")", codes::VIOLATION_CONDITION);
    assert!(v.contains("every member should be on net \"d1\", but #M2:d is on &d2"), "{v}");
    let v = violation("{&d1 &vdd} @ASSERT-NET(global=false, port=true)", codes::VIOLATION_CONDITION);
    assert!(v.contains("every member should be a port of this level, but &d1 is not a port of this level and"), "{v}");
}

#[test]
fn same_different_and_has_compare_the_attributes_of_the_members() {
    let v = violation("{#M1 #M2} @ASSERT-INSTANCE(same=[\"l\"])", codes::VIOLATION_SAME);
    assert!(v.contains("the members should have the same parameter l, but #M1 has \"1u\" and #M2 has \"2u\""), "{v}");
    // some write it and some do not: absent differs from every written value
    let v = violation("{#M1 #M3} @ASSERT-INSTANCE(same=[\"l\"])", codes::VIOLATION_SAME);
    assert!(v.contains("#M1 has \"1u\" and #M3 has no parameter l"), "{v}");
    let v = violation("{#M1 #M1} @ASSERT-INSTANCE(different=[\"name\"])", codes::VIOLATION_DIFFERENT);
    assert!(v.contains("any two members should have a different name, but #M1 is among them more than once"), "{v}");
    let v = violation("{#M1:3 #M2:3} @ASSERT-TERMINAL(different=[\"net\"])", codes::VIOLATION_DIFFERENT);
    assert!(v.contains("have the same net net &s"), "{v}");
    let v = violation("{#M1 #M3} @ASSERT-INSTANCE(has=[\"l\"])", codes::VIOLATION_HAS);
    assert!(v.contains("every member should have a parameter l, but #M3 has none"), "{v}");
    // each check that does not hold is a violation of its own
    let v = violations("{#M1 #M3} @ASSERT-INSTANCE(same=[\"model\", \"l\"], type=\"nmos\")");
    let kinds: Vec<Code> = v.iter().map(|x| x.0).collect();
    assert_eq!(kinds, [codes::VIOLATION_CONDITION, codes::VIOLATION_SAME, codes::VIOLATION_SAME], "{v:?}");
}

#[test]
fn terminals_have_only_net_and_nets_only_name() {
    for (body, attr) in [
        ("{#M1:1 #M2:1} @ASSERT-TERMINAL(same=[\"name\"])", "name"),
        ("{#M1:1 #M2:1} @ASSERT-TERMINAL(different=[\"name\"])", "name"),
        ("{&d1 &d2} @ASSERT-NET(same=[\"net\"])", "net"),
    ] {
        let e = error(body, codes::ASSERT_ATTRIBUTE_NOT_OF_KIND);
        assert!(e.contains(&format!("names the attribute {attr}, but a")), "{e}");
    }
}

#[test]
fn wrong_parameters_of_an_assertion_are_errors() {
    let e = error("{#M1 #M2} @ASSERT-INSTANCE(same=\"model\")", codes::ASSERT_LIST_WRITTEN_AS_STRING);
    assert!(
        e.contains("same= of @ASSERT-INSTANCE is a list of attribute names, written [\"model\"], not \"model\""),
        "{e}"
    );
    error("{#M1 #M2} @ASSERT-INSTANCE(same=[\"\"])", codes::ASSERT_LIST_ITEM_NOT_NAME);
    error("{#M1 #M2} @ASSERT-INSTANCE(terminals=2.5)", codes::ASSERT_TERMINALS_NOT_WHOLE);
    let e = error("{#M1 &d1} @ASSERT-INSTANCE(type=\"nmos\")", codes::ASSERT_MEMBER_WRONG_KIND);
    assert!(e.contains("@ASSERT-INSTANCE checks instances, and &d1 (a net) is not"), "{e}");
    // has is of ASSERT-INSTANCE only
    let r = read(&netlist("0.4", "{&d1 &d2} @ASSERT-NET(has=[\"name\"])"));
    assert_eq!(of(&r, Severity::Error).len(), 1, "{:?}", r.diagnostics());
}

#[test]
fn a_terminal_given_to_assert_net_is_its_net() {
    let v = violation("{#M1:1 #M2:1} @ASSERT-NET(same=[\"name\"])", codes::VIOLATION_SAME);
    assert!(v.contains("&d1 has \"d1\" and &d2 has \"d2\""), "{v}");
}

#[test]
fn an_assertion_in_a_definition_is_checked_for_each_application() {
    let def = "$PAIR = @DEFINE(roles=[[\"left\", \"instance\"], [\"right\", \"instance\"]]) { {?left ?right} @ASSERT-INSTANCE(same=[\"l\"]) };";
    assert_eq!(violations(&format!("{def} #M1(left) #M1(right) @AS $PAIR")), Vec::new());
    let v = violation(&format!("{def} %p = #M1(left) #M2(right) @AS $PAIR"), codes::VIOLATION_SAME);
    assert!(v.contains("@ASSERT-INSTANCE does not hold in %p, a usage of $PAIR at t.sp:"), "{v}");
    // in the body of a disabled usage nothing is checked
    let body = format!("{def} #M1(left) #M2(right) @AS(priority=\"disabled\") $PAIR");
    assert_eq!(violations(&body), Vec::new());
}

#[test]
fn a_disabled_record_checks_nothing() {
    let text = netlist("0.4", "")
        .replace("* @RNL {  }", "* @RNL(priority=\"disabled\") { {#M1 #M2} @ASSERT-INSTANCE(same=[\"l\"]) }");
    let r = read(&text);
    assert_eq!(of(&r, Severity::Violation), Vec::new());
}

#[test]
fn files_of_0_2_and_0_3_have_no_assertions() {
    for version in ["0.2", "0.3"] {
        let r = read(&netlist(version, "{#M1 #M2} @ASSERT-INSTANCE(same=[\"l\"])"));
        let e = of(&r, Severity::Error);
        assert_eq!(e.len(), 1, "{version}: {e:?}");
        assert_eq!(e[0].0, codes::KEYWORD_UNKNOWN, "{version}: {e:?}");
        let said =
            format!("unknown keyword @ASSERT-INSTANCE: the language 0.4 adds it, and this file is of {version} (16.1)");
        assert!(e[0].1.contains(&said), "{e:?}");
        assert_eq!(of(&r, Severity::Violation), Vec::new(), "{version}");
    }
}
