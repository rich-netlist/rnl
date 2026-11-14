//! Symbol-dependent conversions in graphic and assertion input slots (5.2.4).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!(
        "* conversions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {} }}\n.end\n",
        body.replace('\n', "\n* ")
    );
    interpret_file(
        Path::new("conversions.sp"),
        None,
        &MemoryProvider::new().with("conversions.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}
fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}
const ONE_PIN: &str = "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0)";

#[test]
fn every_graphic_point_slot_checks_a_missing_terminal_pin_after_symbols() {
    for graphic in [
        "#R1:2 @CIRCLE(radius=1)",
        "#R1:2 @ELLIPSE(width=1,height=2)",
        "#R1:2 @RECTANGLE(width=1,height=2)",
        "@LINE {#R1:2 (3,0)}",
        "@RECTANGLE {#R1:2 (3,2)}",
        "@CIRCLE {#R1:2 (3,2)}",
        "@ELLIPSE {#R1:2 (3,2)}",
        "@ARC {#R1:2 (1,1) (2,0)}",
        "@BEZIER {#R1:2 (1,0) (2,1) (3,0)}",
    ] {
        let r = read("0.4", &format!("{graphic}; {ONE_PIN}"));
        assert!(errors(&r).iter().any(|e| e.contains("must be points or pins")), "{graphic}: {:?}", errors(&r));
        assert!(r.model.levels[0].shapes.is_empty(), "{graphic}");
        let old = read("0.3", &format!("{graphic}; {ONE_PIN}"));
        assert!(errors(&old).is_empty(), "{graphic}: {:?}", errors(&old));
    }
}

#[test]
fn bound_terminals_and_renderer_standin_pins_still_supply_graphic_points() {
    for body in [
        format!("{ONE_PIN}; #R1:1 @CIRCLE(radius=1)"),
        "#R1 @PLACE(0,0); #R1:1 @CIRCLE(radius=1)".into(),
        format!("{ONE_PIN}; $D= @DEFINE(roles=[\"t\"]) {{}}; %u=#R1:1 @AS $D; %u::t @CIRCLE(radius=1)"),
    ] {
        let r = read("0.4", &body);
        assert!(errors(&r).is_empty(), "{:?}", errors(&r));
        assert_eq!(r.model.levels[0].shapes.len(), 1);
    }
}

#[test]
fn pin_to_terminal_assertions_use_the_final_symbol_binding() {
    let r = read(
        "0.4",
        r#"$S= @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) }; $T= @DEFINE-SYMBOL { %p=(1,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S; #R1::p @ASSERT-TERMINAL(terminal=2,net="b"); #R1 @SET-SYMBOL $T"#,
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
    let r = read(
        "0.4",
        r#"$S= @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; #R1::p @ASSERT-TERMINAL(terminal=2)"#,
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
}

#[test]
fn a_bus_pin_assertion_checks_all_bound_terminals() {
    let r = read(
        "0.4",
        r#"$S= @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=[1,2]) }; #R1 @SET-SYMBOL $S; #R1::p @ASSERT-TERMINAL(same=["net"])"#,
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
}

#[test]
fn unbound_and_usage_pins_do_not_convert_to_host_terminals() {
    for body in [
        r#"$S= @DEFINE-SYMBOL { %p=(0,0) @PIN }; #R1 @SET-SYMBOL $S; #R1::p @ASSERT-TERMINAL"#,
        r#"$S= @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal="n") }; $D= @DEFINE(roles=["n"]) {}; %u=&a @AS $D => @SET-SYMBOL $S; %u::p @ASSERT-TERMINAL"#,
    ] {
        let r = read("0.4", body);
        assert!(errors(&r).iter().any(|e| e.contains("checks terminals")), "{:?}", errors(&r));
    }
}

#[test]
fn graphic_conversion_checks_follow_the_statement_file_in_mixed_includes() {
    for (outer, inner) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let source=format!("* top\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include child.sp\n* @RNL(scope=\"child\") {{ {ONE_PIN}; #R1:2 @CIRCLE(radius=1) }}\n.end\n");
        let child = format!(
            "* child\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n.ends\n"
        );
        let files = MemoryProvider::new().with("top.sp", &source).with("child.sp", &child);
        let r =
            interpret_file(Path::new("top.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert_eq!(
            errors(&r).iter().any(|e| e.contains("must be points or pins")),
            outer == "0.4",
            "{outer}/{inner}: {:?}",
            errors(&r)
        );
    }
}
