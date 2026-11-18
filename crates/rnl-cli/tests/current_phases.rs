//! Current inputs and the generation snapshot (5.2.4, 8.2, 10.4, 15.2).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn source(text: &str) -> Interpretation {
    interpret_file(
        Path::new("current.sp"),
        None,
        &MemoryProvider::new().with("current.sp", text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn read(version: &str, body: &str) -> Interpretation {
    source(&format!(
        "* currents\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {} }}\n.end\n",
        body.replace('\n', "\n* ")
    ))
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn direction(r: &Interpretation, terminal: &str) -> String {
    r.model.levels[0]
        .currents
        .iter()
        .find(|fact| fact.terminal == terminal)
        .map(|fact| fact.direction.clone())
        .unwrap_or_else(|| "unspecified".into())
}

const PINS: &str = r#"$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) => %alias; %n=(0,4) @PIN(terminal=2) }; {#R1 #R2} @SET-SYMBOL $S"#;

#[test]
fn named_pins_use_the_final_symbol_and_the_written_override_order() {
    let r = read(
        "0.4",
        &format!(r#"{PINS}; #R1::p @CURRENT("in"); #R1:1 @CURRENT("out"); #R2::p @CURRENT("out") => @CURRENT("in")"#),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "out");
    assert_eq!(direction(&r, "#R2:1"), "in");
    let r = read(
        "0.4",
        &format!(
            r#"{PINS}; #R1::p @CURRENT("in"); $T = @DEFINE-SYMBOL {{ %p=(0,0) @PIN(terminal=2) }}; #R1 @SET-SYMBOL $T"#
        ),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "unspecified");
    assert_eq!(direction(&r, "#R1:2"), "in");
}

#[test]
fn typed_and_untyped_pin_roles_work_in_ordinary_and_generated_bodies() {
    for role in [r#""t""#, r#"["t","pin"]"#] {
        for target in ["#R1::p", "#R1:1", "#R1 @SELECT-PIN(terminal=1) =>"] {
            let r = read(
                "0.4",
                &format!(r#"{PINS}; $D = @DEFINE(roles=[{role}]) {{ ?t @CURRENT("in") }}; {target} @AS $D"#),
            );
            assert!(errors(&r).is_empty(), "{role}/{target}: {:?}", errors(&r));
            assert_eq!(direction(&r, "#R1:1"), "in", "{role}/{target}");
        }
    }
}

#[test]
fn current_accepts_selected_pins_after_the_ordinary_symbols() {
    let r = read("0.4", &format!(r#"{PINS}; # @SELECT-PIN(terminal=1) => @CURRENT("in")"#));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "in");
    assert_eq!(direction(&r, "#R2:1"), "in");
}

#[test]
fn bus_pins_do_not_split_a_single_current_even_with_one_terminal() {
    for terminals in ["[1]", "[1,2]"] {
        let r = read(
            "0.4",
            &format!(
                r#"$S = @DEFINE-SYMBOL {{ %p=(0,0) @PIN(terminal={terminals}) }}; #R1 @SET-SYMBOL $S => @PLACE(0,0); %saved=#R1::p @CURRENT("in"); %saved @TEXT("kept")"#
            ),
        );
        assert!(errors(&r).iter().any(|e| e.contains("CURRENT takes one host terminal")), "{:?}", errors(&r));
        assert!(r.model.levels[0].currents.is_empty());
        assert!(r.model.levels[0].texts.iter().any(|text| text.markup == "kept"));
    }
}

#[test]
fn unbound_and_usage_pins_have_no_current_terminal() {
    for body in [
        r#"$S = @DEFINE-SYMBOL { %p=(0,0) @PIN }; #R1 @SET-SYMBOL $S; #R1::p @CURRENT("in")"#,
        r#"$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal="n") }; $D = @DEFINE(roles=["n"]) {}; %u=&a @AS $D => @SET-SYMBOL $S; %u::p @CURRENT("in")"#,
    ] {
        let r = read("0.4", body);
        assert!(errors(&r).iter().any(|e| e.contains("CURRENT takes one host terminal")), "{:?}", errors(&r));
        assert!(r.model.levels[0].currents.is_empty());
    }
}

#[test]
fn terminals_need_no_pin_to_have_current() {
    let r = read("0.4", r#"$S = @DEFINE-SYMBOL {}; #R1 @SET-SYMBOL $S; #R1:1 @CURRENT("in")"#);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "in");
}

#[test]
fn current_rules_do_not_read_changes_of_other_rules_in_the_same_phase() {
    let body = r#"#R1:1 @CURRENT("in"); # @SELECT-TERMINAL(current="in") => @CURRENT("out"); # @SELECT-TERMINAL(current="out") => @CURRENT("in")"#;
    let r = read("0.4", body);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "out");
}

#[test]
fn generation_reads_terminal_statements_but_final_assertions_read_pin_currents_too() {
    let r = read(
        "0.4",
        &format!(
            r#"{PINS}; #R1:1 @CURRENT("in"); #R1::p @CURRENT("out"); $D = @DEFINE(roles=["t"]) {{}}; # @SELECT-TERMINAL(current="in") => @AS $D; #R1:1 @ASSERT-TERMINAL(current="out")"#
        ),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(r.problems[0].usages.iter().filter(|usage| usage.definition == "$D").count(), 1);
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
    assert_eq!(direction(&r, "#R1:1"), "out");
}

#[test]
fn a_disabled_record_or_usage_does_not_replace_a_live_current() {
    let r = source("* disabled\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { #R1:1 @CURRENT(\"out\") }\n* @RNL(priority=\"disabled\") { #R1:1 @CURRENT(\"in\") }\n.end\n");
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "out");
    for priority in [r#"(priority="disabled")"#, ""] {
        let suffix = if priority.is_empty() { r#"; %u @PRIORITY("disabled")"# } else { "" };
        let r = read(
            "0.4",
            &format!(
                r#"{PINS}; #R1:1 @CURRENT("out"); $D = @DEFINE(roles=["t"]) {{ ?t @CURRENT("in") }}; %u=#R1::p @AS{priority} $D{suffix}"#
            ),
        );
        assert!(errors(&r).is_empty(), "{:?}", errors(&r));
        assert_eq!(direction(&r, "#R1:1"), "out");
    }
}

#[test]
fn a_role_that_converts_the_pin_to_a_point_cannot_set_current() {
    let r =
        read("0.4", &format!(r#"{PINS}; $D = @DEFINE(roles=[["t","point"]]) {{ ?t @CURRENT("in") }}; #R1::p @AS $D"#));
    assert!(errors(&r).iter().any(|e| e.contains("CURRENT takes one host terminal")), "{:?}", errors(&r));
    assert!(r.model.levels[0].currents.is_empty());
}

#[test]
fn a_failed_pin_role_cannot_fall_back_to_its_original_terminal() {
    let r = read(
        "0.4",
        r#"$S = @DEFINE-SYMBOL {}; #R1 @SET-SYMBOL $S; $D = @DEFINE(roles=[["t","pin"]]) { ?t @CURRENT("in") }; #R1:1 @AS $D"#,
    );
    assert!(!errors(&r).is_empty());
    assert!(r.model.levels[0].currents.is_empty());
}

#[test]
fn a_current_rule_in_a_generated_body_fails_instead_of_recursing() {
    let r = read(
        "0.4",
        &format!(
            r#"{PINS}; $D = @DEFINE(roles=["t"]) {{ ?t @SELECT-TERMINAL => @CURRENT("in") }}; # @SELECT-PIN => @AS $D"#
        ),
    );
    assert!(errors(&r).iter().any(|e| e.contains("generat")), "{:?}", errors(&r));
    assert!(r.model.levels[0].currents.is_empty());
}

#[test]
fn a_written_set_of_pins_keeps_each_current_and_each_alias_is_a_written_handle() {
    let r = read("0.4", &format!(r#"{PINS}; {{#R1::p #R2::p}} @CURRENT("in"); #R1::alias @CURRENT("out")"#));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "out");
    assert_eq!(direction(&r, "#R2:1"), "in");
}

#[test]
fn a_set_role_of_pins_applies_current_to_each_original_member() {
    let r = read(
        "0.4",
        &format!(
            r#"{PINS}; $D = @DEFINE(roles=[["{{ts}}","pin"]]) {{ ?ts @CURRENT("in") }}; {{#R1::p #R2::p}} @AS $D"#
        ),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(direction(&r, "#R1:1"), "in");
    assert_eq!(direction(&r, "#R2:1"), "in");
}

#[test]
fn generated_symbol_rules_can_read_ordinary_pin_currents() {
    for current in [r#"#R1::p @CURRENT("out")"#, r#"#R1 @SELECT-PIN(terminal=1) => @CURRENT("out")"#] {
        let r = read(
            "0.4",
            &format!(
                r#"{PINS};
            $G = @DEFINE(roles=["t"]) {{}};
            $U = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=1) }};
            {current};
            #R1 @SELECT-TERMINAL(terminal=1) => @AS $G;
            # @SELECT-TERMINAL(current="out") => @SELECT-ANCESTOR-USAGE $G => @SET-SYMBOL $U"#
            ),
        );
        assert!(errors(&r).is_empty(), "{current}: {:?}", errors(&r));
        let usage = r.problems[0].usages.iter().find(|usage| usage.definition == "$G").unwrap();
        let object = usage.object.unwrap();
        assert!(r.problems[0].objects[object].symbol.is_some(), "{current}");
    }
}

#[test]
fn final_current_checks_include_pin_facts_and_ignore_disabled_ones() {
    let r = read("0.4", &format!(r#"{PINS}; #R1::p @CURRENT("in"); #R2::p @CURRENT("in")"#));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let violations: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Violation).collect();
    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.contains("current of &a goes one way"));
    let r = read(
        "0.4",
        &format!(
            r#"{PINS}; #R1::p @CURRENT("in"); $D = @DEFINE(roles=["t"]) {{ ?t @CURRENT("in") }}; #R2::p @AS(priority="disabled") $D"#
        ),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
    assert_eq!(r.model.levels[0].currents.len(), 1);
}

#[test]
fn current_rule_phases_hold_in_a_scoped_record_on_an_included_level() {
    let top = "* top\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include child.sp\n* @RNL(scope=\"child\") { #R1:1 @CURRENT(\"in\"); # @SELECT-TERMINAL(current=\"in\") => @CURRENT(\"out\"); # @SELECT-TERMINAL(current=\"out\") => @CURRENT(\"in\") }\n.end\n";
    let child = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a b\nR1 a b 1k\n.ends\n";
    let files = MemoryProvider::new().with("current.sp", top).with("child.sp", child);
    let r =
        interpret_file(Path::new("current.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let level = r.model.levels.iter().find(|level| level.name == "child").unwrap();
    assert_eq!(level.currents.iter().find(|fact| fact.terminal == "#R1:1").unwrap().direction, "out");
}
