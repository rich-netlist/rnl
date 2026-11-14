//! The diagnostics of bundles, markup and aliases (chapters 11-17), each given where its text is,
//! in a file of 0.4.
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use std::path::Path;

/// The kinds, severities and lines of the reading of a netlist of 0.4 with `header` in its header
/// record, line 2, and `statements` in a record on line 9.
fn read(header: &str, statements: &str) -> Vec<(Code, Severity, u32)> {
    let text = format!(
        "* diagnostics\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"){header} }}\nR1 a b 1k\nR2 b 0 1k\nX1 a b sub\n.subckt sub p n\nR9 p n 1k\n.ends sub\n* @RNL {{ {statements} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("main.sp", &text);
    let r = interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() }).unwrap();
    r.diagnostics().iter().map(|d| (d.code, d.severity, d.loc.as_ref().map_or(0, |l| l.line))).collect()
}

#[test]
fn bundles_take_host_instances_drawn_together() {
    for (code, severity, statements) in [
        // a member of a bundle is not drawn on its own (11.8)
        (codes::BUNDLE_MEMBER_DRAWN_ALONE, Severity::Error, "{#R1 #R2} @BUNDLE; #R1 @PLACE(0, 0)"),
        // a member in several bundles goes to the later one, and an emptied bundle is not drawn
        (codes::BUNDLE_MEMBER_IN_SEVERAL_BUNDLES, Severity::Hint, "{#R1 #R2} @BUNDLE; {#R1 #X1} @BUNDLE"),
        (codes::BUNDLE_EMPTY, Severity::Warning, "{#R1} @BUNDLE; {#R1 #R2} @BUNDLE"),
        // the members are host instances
        (codes::KEYWORD_BUNDLE_MEMBER_NOT_INSTANCE, Severity::Error, "{&a &b} @BUNDLE"),
    ] {
        let found = read("", statements);
        assert!(found.contains(&(code, severity, 9)), "{code:?}: {found:?}");
    }
}

#[test]
fn unbalanced_braces_in_markup_are_shown_as_written() {
    // 13.6
    let found = read("", "#R1 @TEXT(\"a{b\")");
    assert!(found.contains(&(codes::MARKUP_UNBALANCED_BRACES, Severity::Warning, 9)), "{found:?}");
}

#[test]
fn use_aliases_cannot_have_an_alias() {
    // the aliases are not known before it is read (16.5)
    let found = read("; @ALIAS(\"@UA\", \"@USE-ALIASES\")", "#R1 @PLACE(0, 0)");
    assert!(found.contains(&(codes::ALIAS_OF_USE_ALIASES, Severity::Error, 2)), "{found:?}");
}
