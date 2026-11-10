use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;

fn rewritten(statement: &str) -> rnl_edit::upgrade::Upgrade {
    let source = format!(
        "* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a 0 1k\n* @RNL {{ {statement} }}\n.end\n"
    );
    upgrade("t.sp", &MemoryProvider::new().with("t.sp", &source), &UpgradeOptions::default()).unwrap()
}

#[test]
fn wired_text_filters_electrical_terminals_before_selecting_their_pins() {
    let result = rewritten("# @SELECT-TERMINAL(net=\"a\", wired=false) => @TEXT(\"${net}\", key=\"netname\")");
    assert!(result.files["t.sp"]
        .contains("@SELECT-TERMINAL(net=\"a\", recursive=true) => @SELECT-PIN(wired=false, recursive=true) => @TEXT"));
    assert!(!result.report.files[0].unconverted.iter().any(|u| u.rule == "pin-selection"));
}

#[test]
fn local_selection_and_wired_true_remain_explicit() {
    let result = rewritten("# @SELECT-TERMINAL(recursive=false, wired=true) => @TEXT(\"wired\")");
    assert!(
        result.files["t.sp"].contains("@SELECT-TERMINAL(recursive=false) => @SELECT-PIN(wired=true, recursive=false)")
    );
}

#[test]
fn electrical_consumers_keep_the_original_selection_and_a_reason() {
    let result = rewritten("# @SELECT-TERMINAL(wired=false) => @CURRENT(\"in\")");
    assert!(result.files["t.sp"].contains("@SELECT-TERMINAL(wired=false, recursive=true)"));
    assert!(!result.files["t.sp"].contains("@SELECT-PIN(wired=false"));
    assert!(result.report.files[0].unconverted.iter().any(|u| u.rule == "pin-selection"));
}

#[test]
fn bound_selection_is_not_changed_to_another_kind() {
    let result = rewritten("%terminals = # @SELECT-TERMINAL(wired=false); %terminals @TEXT(\"note\")");
    assert!(result.files["t.sp"].contains("%terminals = # @SELECT-TERMINAL(wired=false, recursive=true)"));
    assert!(result.report.files[0].unconverted.iter().any(|u| u.rule == "pin-selection"));
}

#[test]
fn text_without_a_wired_filter_still_uses_only_drawn_terminals() {
    for source in ["# @SELECT-TERMINAL => @TEXT(\"${net}\")", "# @SELECT-TERMINAL(net=\"a\") => @TEXT(\"${net}\")"] {
        let r = rewritten(source);
        assert!(r.files["t.sp"].contains("=> @SELECT-PIN(recursive=true) => @TEXT"));
        assert!(!r.report.files[0].unconverted.iter().any(|u| u.rule == "terminal-selection"));
    }
}
#[test]
fn electrical_terminal_consumers_without_wired_need_membership_review() {
    let r = rewritten("# @SELECT-TERMINAL => @CURRENT(\"in\")");
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "terminal-selection"));
    assert!(r.files["t.sp"].contains("@SELECT-TERMINAL(recursive=true) => @CURRENT"));
}
#[test]
fn selection_traversal_reports_the_change_to_hidden_terminals() {
    let r = rewritten("&a @SELECT-INSTANCE => @TEXT(\"part\")");
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "terminal-traversal"));
}
