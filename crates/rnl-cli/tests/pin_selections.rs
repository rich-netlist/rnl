//! Circuit terminals, drawing pins and symbol-dependent selection (10.1).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::Filler;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const SYMBOL: &str = r#"
    $S = @DEFINE-SYMBOL {
        %p = (0,0) @PIN(terminal=1);
        %q = (1,0) @PIN(terminal=1);
        %b = (2,0) @PIN(terminal=[1,2]);
        %free = (3,0) @PIN
    };
    #R1 @SET-SYMBOL $S
"#;

fn read(version: &str, body: &str) -> Interpretation {
    let body = body.replace('\n', "\n* ");
    let source = format!("* selections\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b c 2k\n* @RNL {{ $PROBE = @DEFINE(roles=[\"x\"]) {{}}; {body} }}\n.end\n");
    read_source(&source)
}

fn read_source(source: &str) -> Interpretation {
    interpret_file(
        Path::new("selection.sp"),
        None,
        &MemoryProvider::new().with("selection.sp", source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn count(r: &Interpretation) -> usize {
    r.problems[0].usages.iter().filter(|u| u.definition == "$PROBE").count()
}

fn selected(body: &str, selection: &str) -> Interpretation {
    let r = read("0.4", &format!("{body}; {selection} => @AS $PROBE"));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

#[test]
fn terminal_scope_selects_every_independent_bound_pin() {
    for (scope, expected) in
        [("#R1", 4), ("#R1:1", 3), ("#R1:2", 1), ("&a", 3), ("&b", 2), ("#R1::p", 1), ("#R1::p::position", 0)]
    {
        let r = selected(SYMBOL, &format!("{scope} @SELECT-PIN"));
        assert_eq!(count(&r), expected, "{scope}");
        assert!(r.problems[0]
            .usages
            .iter()
            .filter(|u| u.definition == "$PROBE")
            .all(|u| matches!(u.roles[0].members[..], [Filler::Pin { .. }])));
    }
}

#[test]
fn level_scope_includes_the_pins_of_default_block_symbols() {
    // #R2 has the default block symbol, with a pin for each terminal; it is drawn (9.2.6, 10.1)
    assert_eq!(count(&selected(SYMBOL, "# @SELECT-PIN")), 6);
    assert_eq!(count(&selected("", "# @SELECT-PIN")), 4);
    assert_eq!(count(&selected(SYMBOL, "# @SELECT-INSTANCE(drawn=true)")), 2);
    assert_eq!(count(&selected(SYMBOL, "# @SELECT-INSTANCE(drawn=false)")), 0);
    assert_eq!(count(&selected(SYMBOL, r#"# @SELECT-INSTANCE(symbol="*")"#)), 1);
}

#[test]
fn pin_conditions_keep_unbound_and_bus_pins_distinct() {
    for (condition, expected) in [
        ("terminal=1", 4),
        ("terminal=2", 2),
        ("net=\"a\"", 3),
        ("net=\"b\"", 2),
        ("name=\"r*\"", 6),
        ("name=\"R2\"", 2),
        ("wired=false", 6),
    ] {
        assert_eq!(count(&selected(SYMBOL, &format!("# @SELECT-PIN({condition})"))), expected, "{condition}");
    }
}

#[test]
fn explicit_wire_counts_only_the_physical_pin_it_touches() {
    for (point, expected) in [("#R1::p", 1), ("#R1:1", 1), ("#R1::b", 1), ("#R1::p::position", 0)] {
        let body = format!("{SYMBOL}; &a @WIRE {{{point} (5,5)}}");
        assert_eq!(count(&selected(&body, "# @SELECT-PIN(wired=true)")), expected, "{point}");
    }
}

#[test]
fn terminal_selection_reads_the_circuit_even_when_no_pin_is_drawn() {
    let body = r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S"#;
    assert_eq!(count(&selected(body, "#R1 @SELECT-TERMINAL")), 2);
    assert_eq!(count(&selected(body, "&b @SELECT-INSTANCE")), 2);
    assert_eq!(count(&selected(body, "#R1 @SELECT-NET")), 2);
    let r = read("0.3", &format!("{body}; #R1 @SELECT-TERMINAL => @AS $PROBE"));
    assert_eq!(count(&r), 1);
}

#[test]
fn pin_scope_selects_all_bound_host_terminals() {
    assert_eq!(count(&selected(SYMBOL, "#R1::b @SELECT-TERMINAL")), 2);
    assert_eq!(count(&selected(SYMBOL, "#R1::free @SELECT-TERMINAL")), 0);
}

#[test]
fn pin_selection_preserves_scope_order_and_deduplicates_physical_pins() {
    let r = selected(SYMBOL, "{#R1::q #R1:1 #R1::p} @SELECT-PIN");
    let pins: Vec<_> = r.problems[0]
        .usages
        .iter()
        .filter(|u| u.definition == "$PROBE")
        .map(|u| match u.roles[0].members[0] {
            Filler::Pin { pin, .. } => pin,
            _ => panic!(),
        })
        .collect();
    assert_eq!(pins, [1, 0, 2]);
}

#[test]
fn usage_scope_selects_own_pins_unless_a_role_is_requested() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=["x"]) {{}};
        $U = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal="x") }};
        %u = #R1::p @AS $D => @SET-SYMBOL $U"#
    );
    let own = selected(&body, "%u @SELECT-PIN");
    let role = selected(&body, "%u @SELECT-PIN(role=\"x\")");
    assert_eq!(count(&own), 1);
    assert_eq!(count(&role), 1);
    let filler = |r: &Interpretation| {
        r.problems[0].usages.iter().find(|u| u.definition == "$PROBE").unwrap().roles[0].members[0].clone()
    };
    assert_ne!(filler(&own), filler(&role));
    assert_eq!(count(&selected(&body, "&a @SELECT-PIN")), 4);
    assert_eq!(count(&selected(&body, "%u @SELECT-TERMINAL")), 0);
}

#[test]
fn usage_role_members_are_not_reinterpreted_as_scopes() {
    for filler in ["#R1", "#R1:1", "&a"] {
        let body = format!(r#"{SYMBOL}; $D = @DEFINE(roles=["x"]) {{}}; %u = {filler} @AS $D"#);
        assert_eq!(count(&selected(&body, "%u @SELECT-PIN")), 0, "{filler}");
    }
}

#[test]
fn symbol_condition_compares_the_assignment_name_not_the_definition_identity() {
    let body = format!("{SYMBOL}; $OTHER = $S; #R1 @SET-SYMBOL $OTHER");
    assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="$OTHER")"#)), 1);
    assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="$S")"#)), 0);
    assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="$OTH*")"#)), 1);
    assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="$other")"#)), 0);
}

#[test]
fn percent_and_anonymous_symbols_are_drawn_but_match_no_dollar_name() {
    for assignment in ["%s = $S; #R1 @SET-SYMBOL %s", "#R1 @SET-SYMBOL { @DEFINE-SYMBOL { (0,0) @PIN(terminal=1)}}"] {
        let body = format!("{SYMBOL}; {assignment}");
        assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="*")"#)), 0);
        assert_eq!(count(&selected(&body, "# @SELECT-INSTANCE(drawn=true)")), 2);
    }
}

#[test]
fn symbol_conditions_apply_to_usages_and_instance_assertions() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=["x"]) {{}};
        $U = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal="x") }};
        %u = &a @AS $D => @SET-SYMBOL $U; %v = &b @AS $D"#
    );
    assert_eq!(count(&selected(&body, r#"# @SELECT-USAGE(symbol="$U")"#)), 1);
    assert_eq!(count(&selected(&body, "# @SELECT-USAGE(drawn=false)")), 1);
    let r = read(
        "0.4",
        &format!(r#"{SYMBOL}; #R1 @ASSERT-INSTANCE(symbol="$S",drawn=true); #R2 @ASSERT-INSTANCE(drawn=true)"#),
    );
    assert!(
        !r.diagnostics().iter().any(|d| matches!(d.severity, Severity::Error | Severity::Violation)),
        "{:?}",
        r.diagnostics()
    );
    let r =
        read("0.4", &format!(r#"{SYMBOL}; #R1 @ASSERT-INSTANCE(symbol="$OTHER"); #R2 @ASSERT-INSTANCE(drawn=false)"#));
    assert_eq!(r.diagnostics().iter().filter(|d| d.severity == Severity::Violation).count(), 2);
}

#[test]
fn symbol_queue_rejects_direct_and_nested_symbol_dependent_selections() {
    for selection in [
        "# @SELECT-PIN",
        "# @SELECT-INSTANCE(drawn=false)",
        "# @SELECT-USAGE(symbol=\"$S\")",
        "#R1::p @SELECT-TERMINAL",
        "{%s} @UNION",
    ] {
        let body = format!("{SYMBOL}; %s = # @SELECT-PIN; {selection} => @SET-SYMBOL $S");
        let r = read("0.4", &body);
        assert!(
            r.diagnostics().iter().any(|d| d.message.contains("symbol queue")),
            "{selection}: {:?}",
            r.diagnostics()
        );
    }
}

#[test]
fn terminal_count_condition_is_exact_and_rejects_fractional_counts() {
    assert_eq!(count(&selected(SYMBOL, "# @SELECT-INSTANCE(terminals=2)")), 2);
    assert_eq!(count(&selected(SYMBOL, "# @SELECT-INSTANCE(terminals=3)")), 0);
    for n in ["-1", "1.5"] {
        let r = read("0.4", &format!("# @SELECT-INSTANCE(terminals={n})"));
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    }
}

#[test]
fn new_words_are_version_gated_and_terminal_wired_is_removed() {
    for statement in ["# @SELECT-PIN", "# @SELECT-INSTANCE(symbol=\"$S\")", "# @SELECT-USAGE(drawn=true)"] {
        let r = read("0.3", statement);
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{statement}");
    }
    let r = read("0.4", "# @SELECT-TERMINAL(wired=false)");
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn pin_role_conversions_are_visible_to_generation_selections() {
    let body = format!(r#"{SYMBOL}; $D = @DEFINE(roles=[["x","pin"]]) {{}}; %u = #R1:1 @AS $D"#);
    assert_eq!(count(&selected(&body, "%u @SELECT-PIN")), 1);
    assert_eq!(count(&selected(&body, "%u::x @SELECT-PIN")), 1);
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=[["{{x}}","pin"]]) {{}};
        %s = {{#R1:1}}; %u = %s @AS $D; #R1:2 @ADD-TO %s"#
    );
    assert_eq!(count(&selected(&body, "%u @SELECT-PIN")), 1); // Both convert to the same final bus pin.
}

#[test]
fn standard_positions_and_exact_role_kinds_do_not_read_symbols_in_the_queue() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=[["x",["terminal","pin"]],"other"]) {{}};
        %u = #R2:1(x) #R1::p(other) @AS $D;
        %u::x @SELECT-INSTANCE => @SET-SYMBOL $S;
        %u::position @SELECT-INSTANCE => @SET-SYMBOL $S"#
    );
    assert_eq!(count(&selected(&body, "#R2 @SELECT-PIN")), 4);
}

#[test]
fn bundles_select_their_own_pins_and_all_member_networks() {
    let body = r#"$S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1); %q = (1,0) @PIN(terminal=1) };
        %b = {#R1 #R2} @BUNDLE => @SET-SYMBOL $S"#;
    assert_eq!(count(&selected(body, "# @SELECT-PIN")), 2);
    assert_eq!(count(&selected(body, "%b @SELECT-PIN(net=\"b\")")), 2);
    assert_eq!(count(&selected(body, "%b @SELECT-PIN(terminal=1)")), 2);
}

#[test]
fn symbol_queue_can_select_usage_identity_without_reading_its_pin_roles() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=["x"]) {{}};
        $U = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal="x") }};
        %u = #R1::p @AS $D; %u @SELECT-USAGE $D => @SET-SYMBOL $U"#
    );
    let r = selected(&body, "%u @SELECT-PIN");
    assert_eq!(count(&r), 1);
}

#[test]
fn symbol_queue_checks_only_the_requested_usage_role() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=["host","pin"]) {{}};
        %u = #R2(host) #R1::p(pin) @AS $D;
        %u @SELECT-INSTANCE(role="host") => @SET-SYMBOL $S"#
    );
    assert_eq!(count(&selected(&body, "#R2 @SELECT-PIN")), 4);
}

#[test]
fn late_set_edits_cannot_hide_a_symbol_dependency_from_the_queue() {
    let r = read(
        "0.4",
        &format!(
            r#"{SYMBOL}; %s = {{}};
        %s @SET-SYMBOL $S; # @SELECT-INSTANCE(drawn=false) => @ADD-TO %s"#
        ),
    );
    assert!(r.diagnostics().iter().any(|d| d.message.contains("symbol queue")), "{:?}", r.diagnostics());
}

#[test]
fn dollar_aliases_compare_as_their_original_assignment_name() {
    let r = read_source(
        r#"* aliases
* @RNL(type="header") { @VERSION("0.4"); @ALIAS("$ALIAS","$S") }
R1 a b 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
* $PROBE = @DEFINE(roles=["x"]) {};
* #R1 @SET-SYMBOL $ALIAS;
* #R1 @ASSERT-INSTANCE(symbol="$S", drawn=true);
* # @SELECT-INSTANCE(symbol="$S") => @AS $PROBE;
* # @SELECT-INSTANCE(symbol="$ALIAS") => @AS $PROBE
* }
.end
"#,
    );
    assert!(
        !r.diagnostics().iter().any(|d| matches!(d.severity, Severity::Error | Severity::Violation)),
        "{:?}",
        r.diagnostics()
    );
    assert_eq!(count(&r), 1);
}

#[test]
fn block_symbol_assignments_keep_their_written_name() {
    let r = read_source(
        r#"* blocks
* @RNL(type="header") { @VERSION("0.4") }
* @RNL { $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; $BLOCK = $S; $PROBE = @DEFINE(roles=["x"]) {} }
.subckt cell a b
* @RNL { # @SET-SYMBOL $BLOCK }
R1 a b 1k
.ends cell
X1 a b cell
* @RNL { #X1 @ASSERT-INSTANCE(symbol="$BLOCK",drawn=true); #X1 @SELECT-INSTANCE(symbol="$BLOCK") => @AS $PROBE }
.end
"#,
    );
    assert!(
        !r.diagnostics().iter().any(|d| matches!(d.severity, Severity::Error | Severity::Violation)),
        "{:?}",
        r.diagnostics()
    );
    assert_eq!(count(&r), 1);
}

#[test]
fn mapped_symbols_and_losing_rules_do_not_change_the_assignment_name() {
    let body = format!(
        r#"{SYMBOL}; $DIRECT = $S; $RULE = $S;
        #R1 @SET-SYMBOL(terminals=[2,1]) $DIRECT;
        # @SELECT-INSTANCE => @SET-SYMBOL $RULE"#
    );
    assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="$DIRECT")"#)), 1);
    assert_eq!(count(&selected(&body, r#"# @SELECT-INSTANCE(symbol="$RULE")"#)), 1);
}

#[test]
fn selections_use_all_members_of_late_edited_usage_set_roles() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=["{{x}}"]) {{}};
        %s = {{#R1::p}}; %u = %s @AS $D; #R1::q @ADD-TO %s"#
    );
    assert_eq!(count(&selected(&body, "%u @SELECT-PIN")), 2);
}

#[test]
fn a_direct_set_symbol_assignment_keeps_its_input_for_the_next_segment() {
    let r = read("0.4", &format!(r#"{SYMBOL}; {{#R1 #R2}} @SET-SYMBOL $S => @AS $PROBE"#));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert_eq!(count(&r), 2);
    assert!(r.problems[0].usages.iter().filter(|u| u.definition == "$PROBE").all(|u| !u.generated));
}

#[test]
fn generated_bodies_can_assign_symbols_to_a_set_of_new_usages() {
    let r = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal="n") };
        $INNER = @DEFINE(roles=["n"]) {};
        $D = @DEFINE(roles=["n"]) { %u = ?n @AS $INNER; {%u} @SET-SYMBOL $S };
        # @SELECT-NET => @AS $D
    "#,
    );
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert_eq!(
        r.model.levels[0]
            .objects
            .iter()
            .filter(|o| o.definition.as_deref() == Some("$INNER") && o.symbol.is_some())
            .count(),
        3
    );
}

#[test]
fn generated_wires_do_not_change_the_selection_snapshot() {
    let body = format!(
        r#"{SYMBOL}; $D = @DEFINE(roles=[["p","pin"]]) {{ ?p::net @WIRE {{?p (8,8)}} }};
        # @SELECT-PIN(net="a") => @AS $D"#
    );
    assert_eq!(count(&selected(&body, "# @SELECT-PIN(wired=false)")), 6);
}

#[test]
fn a_wire_that_fails_later_still_counts_as_written() {
    let body = format!(
        r#"{SYMBOL}; $EMPTY = @DEFINE-SYMBOL {{}}; #R2 @SET-SYMBOL $EMPTY;
        &a @WIRE {{#R1::p #R2:1}}; # @SELECT-PIN(wired=true) => @AS $PROBE"#
    );
    let r = read("0.4", &body);
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    assert_eq!(count(&r), 1);
}

#[test]
fn pin_selection_defaults_to_its_level_but_keeps_an_explicit_recursive_pipe() {
    for (selection, expected) in [
        ("# @SELECT-PIN", 0),
        ("# @SELECT-PIN(recursive=true)", 1),
        ("# @SELECT-INSTANCE(recursive=true) => @SELECT-PIN", 1),
        ("# @SELECT-INSTANCE(recursive=false) => @SELECT-PIN(recursive=true)", 1),
        ("# @SELECT-INSTANCE(recursive=true) => @SELECT-PIN(recursive=false)", 1),
    ] {
        let r = read_source(&format!(
            r#"* levels
* @RNL(type="header") {{ @VERSION("0.4") }}
* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; $PROBE = @DEFINE(roles=["x"]) {{}} }}
.subckt cell a b
R1 a b 1k
* @RNL {{ #R1 @SET-SYMBOL $S }}
.ends cell
X1 a b cell
* @RNL {{ {selection} => @AS $PROBE }}
.end
"#
        ));
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{selection}: {:?}", r.diagnostics());
        let child = r.problems.iter().find(|p| p.name == "cell").unwrap();
        assert_eq!(child.usages.iter().filter(|u| u.definition == "$PROBE").count(), expected, "{selection}");
    }
}

#[test]
fn a_symbol_binding_a_converted_point_role_is_not_drawn_in_the_selection_snapshot() {
    let r = read(
        "0.4",
        &format!(
            r#"{SYMBOL};
        $D = @DEFINE(roles=[["x","point"]]) {{}};
        $U = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal="x") }};
        %u = #R1:1 @AS $D => @SET-SYMBOL $U;
        # @SELECT-USAGE(drawn=true) $D => @AS $PROBE
    "#
        ),
    );
    assert!(
        r.diagnostics()
            .iter()
            .any(|d| d.message.contains("filled with the position of") && d.message.contains("(point)")),
        "{:?}",
        r.diagnostics()
    );
    assert_eq!(count(&r), 0);
}

#[test]
fn a_role_member_does_not_take_the_scope_step_of_a_later_direct_occurrence() {
    for (filler, keyword, expected) in [
        ("#R1", "SELECT-TERMINAL", vec![1, 1, 0, 0]),
        ("#R1:1", "SELECT-INSTANCE", vec![1, 0]),
        ("#R1", "SELECT-PIN", vec![1, 1, 1, 1, 0, 0, 0, 0]),
    ] {
        let r = selected(
            &format!("{SYMBOL}; #R2 @SET-SYMBOL $S; $D = @DEFINE(roles=[\"x\"]) {{}}; %u = {filler} @AS $D"),
            &format!("{{%u #R2 #R1}} @{keyword}"),
        );
        let owners: Vec<_> = r.problems[0]
            .usages
            .iter()
            .filter(|u| u.definition == "$PROBE")
            .map(|u| match u.roles[0].members[0] {
                Filler::Object(object) | Filler::Terminal { object, .. } | Filler::Pin { object, .. } => object,
                ref other => panic!("unexpected member: {other:?}"),
            })
            .collect();
        assert_eq!(owners, expected, "{keyword}");
    }
}

#[test]
fn a_late_pin_conversion_failure_keeps_the_usage_in_the_generation_snapshot() {
    let r = read(
        "0.4",
        r#"$S = @DEFINE-SYMBOL { (0,0) (1,1) @LINE }; #R1 @SET-SYMBOL $S;
        $D = @DEFINE(roles=[["x","pin"]]) {};
        $U = @DEFINE-SYMBOL { (0,0) @PIN(terminal="x") };
        %u = #R1:1 @AS $D => @SET-SYMBOL $U;
        $WITNESS = @DEFINE(roles=["x"],parameters=[["radius","bad"]]) { (0,0) @CIRCLE(radius=?radius) };
        # @SELECT-USAGE(drawn=true) $D => @AS $WITNESS"#,
    );
    assert!(r.diagnostics().iter().any(|d| d.message.contains("cannot convert")), "{:?}", r.diagnostics());
    // The parameter keeps the error dependent on this application's values, rather than a
    // bad literal checked in an unused definition. The body ran before the missing-pin check:
    // its own error is
    // retained even though the invalid role later makes both usages fail (8.2).
    assert!(
        r.diagnostics().iter().any(|d| d.severity == Severity::Error && d.origin.contains("$WITNESS")),
        "{:?}",
        r.diagnostics()
    );
}
