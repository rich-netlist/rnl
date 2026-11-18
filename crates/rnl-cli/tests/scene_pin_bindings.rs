//! A usage symbol may bind only nets, terminals and pins to its pins (12.1).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let body = body.replace('\n', "\n* ");
    let source = format!("* role bindings\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n");
    interpret_file(
        Path::new("bindings.sp"),
        None,
        &MemoryProvider::new().with("bindings.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

#[test]
fn a_wrong_bound_role_removes_only_the_usage_symbol() {
    for filler in ["#R1", "(2,3)", "#R1::position"] {
        let body = format!(
            r#"
            $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal="r"); (0,0) @CIRCLE(radius=1) }};
            $D = @DEFINE(roles=["r"]) {{ (1,1) @CIRCLE(radius=2) }};
            %u = {filler} @AS $D;
            %u @SET-SYMBOL $S => @PLACE(4,5)
        "#
        );
        let r = read("0.4", &body);
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
        assert_eq!(errors.len(), 1, "{filler}: {errors:?}");
        assert_eq!(errors[0].code, codes::ROLE_PIN_FILLED_WITH_OTHER_KIND);
        assert!(errors[0].message.contains("the symbol is given at bindings.sp:"));
        assert_eq!(r.problems[0].usages.len(), 1);
        assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_none()));
        assert!(r.model.levels[0].failed_usages.is_empty());
        assert_eq!(r.model.levels[0].shapes.len(), 1);
    }
}

#[test]
fn electrical_fillers_and_set_roles_keep_the_symbol() {
    for (role, filler) in [("r", "&a"), ("r", "#R1:1"), ("r", "#R1::p"), ("{r}", "{&a #R1:2 #R1::p}")] {
        let body = format!(
            r#"
            $S = @DEFINE-SYMBOL {{ %p = (0,0) @PIN(terminal=1) }};
            $V = @DEFINE-SYMBOL {{ %out = (0,0) @PIN(terminal="r") }};
            $D = @DEFINE(roles=["{role}"]) {{}};
            %u = {filler} @AS $D;
            %u @SET-SYMBOL $V => @PLACE(4,5);
            #R1 @SET-SYMBOL $S
        "#
        );
        let r = read("0.4", &body);
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{filler}: {:?}", r.diagnostics());
        assert_eq!(r.problems[0].usages.len(), 1);
        assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_some()));
    }
}

#[test]
fn one_wrong_set_member_fails_the_whole_symbol_assignment() {
    let r = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal="r"); (1,0) @PIN(terminal="r") };
        $D = @DEFINE(roles=["{r}"]) {};
        %u = {&a #R1 &b} @AS $D;
        %u @SET-SYMBOL $S
    "#,
    );
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::ROLE_PIN_FILLED_WITH_OTHER_KIND).count(), 2);
    assert_eq!(r.problems[0].usages.len(), 1);
    assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_none()));
}

#[test]
fn a_missing_binding_warns_and_a_role_without_a_pin_is_unrestricted() {
    let r = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal="missing"); %free = (1,0) @PIN };
        $D = @DEFINE(roles=["r"]) {};
        %u = #R1 @AS $D;
        %u @SET-SYMBOL $S
    "#,
    );
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert!(r.diagnostics().iter().any(|d| d.code == codes::PIN_TERMINAL_NAMES_NO_ROLE));
    assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_some()));
}

#[test]
fn generated_usages_check_the_second_symbol_queue_too() {
    let r = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal="r") };
        $D = @DEFINE(roles=["r"]) { ? @SET-SYMBOL $S };
        # @SELECT-INSTANCE => @AS $D
    "#,
    );
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::ROLE_PIN_FILLED_WITH_OTHER_KIND).count(), 1);
    assert_eq!(r.problems[0].usages.len(), 1);
    assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_none()));
}

#[test]
fn a_named_text_role_does_not_become_an_unconnected_pin() {
    let r = read(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal="r") };
        $D = @DEFINE(roles=["r"]) {};
        %u = #R1::label @AS $D;
        %u @SET-SYMBOL $S;
        #R1 @TEXT("label",key="label")
    "#,
    );
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::ROLE_PIN_FILLED_WITH_OTHER_KIND).count(), 1);
    assert_eq!(r.problems[0].usages.len(), 1);
    assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_none()));
}

#[test]
fn a_late_rejected_symbol_drops_its_labels_and_checks_explicit_text_separately() {
    let r = read(
        "0.4",
        r#"
        $R = @DEFINE-SYMBOL { ? @TEXT("source",key="label") };
        $S = @DEFINE-SYMBOL {
            (0,0) @PIN(terminal="r");
            ? @TEXT("symbol plain");
            ? @TEXT("symbol keyed",key="note");
            ? @TEXT("symbol only",key="orphan")
        };
        $D = @DEFINE(roles=["r"]) {};
        %u = #R1::label @AS $D;
        %u @SET-SYMBOL $S => @PLACE(4,5);
        %u @TEXT("explicit",key="note");
        %u::position @TEXT("point label");
        #R1 @SET-SYMBOL $R => @PLACE(0,0)
    "#,
    );
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::ROLE_PIN_FILLED_WITH_OTHER_KIND).count(), 1);
    assert!(r.problems[0].objects.iter().filter(|o| o.usage.is_some()).all(|o| o.symbol.is_none()));
    let texts: Vec<_> = r.model.levels[0].texts.iter().map(|text| text.markup.as_str()).collect();
    assert!(texts.contains(&"source"), "{texts:?}");
    assert!(texts.contains(&"point label"), "{texts:?}");
    // The explicit layer survives the rejected symbol and gets its own normal check:
    // TEXT on a usage without a symbol fails, while TEXT on its point remains valid.
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::TEXT_USAGE_NO_SYMBOL).count(), 1);
    assert!(texts.iter().all(|text| !text.starts_with("symbol")), "{texts:?}");
}
