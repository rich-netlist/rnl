//! Markup warnings identify the value that supplies the text, in the source's character coordinates.
use std::path::Path;

use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, aliases: &str, body: &str, spectre: bool) -> (Interpretation, String) {
    let marker = if spectre { "//" } else { "*" };
    let path = if spectre { "main.scs" } else { "main.sp" };
    let host = if spectre { "R1 (a b) resistor r=1k" } else { "R1 a b 1k" };
    let line = format!("{marker} @RNL {{ {body} }}");
    let source = format!(
        "{marker} markup\n{marker} @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {aliases} }}\n{host}\n{line}\n"
    );
    let r = rnl_cli::interpret_file(
        Path::new(path),
        None,
        &MemoryProvider::new().with(path, &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    (r, line)
}

fn column(line: &str, needle: &str) -> usize {
    line[..line.find(needle).unwrap()].chars().count() + 1
}

#[test]
fn literal_text_positions_handle_names_comments_and_unicode() {
    for version in ["0.3", "0.4"] {
        for spectre in [false, true] {
            for body in [
                r#"/* 中文 */ (0,0) @TEXT("\zzzz")"#,
                r#"(0,0) @TEXT(text = /* value */ "\zzzz",size=2)"#,
                r#"(0,0) @TEXT(size=2,"text" /* name */ = "\zzzz")"#,
            ] {
                let (r, line) = read(version, "", body, spectre);
                assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
                let warnings: Vec<_> =
                    r.diagnostics().into_iter().filter(|d| d.code == codes::MARKUP_UNKNOWN_COMMAND).collect();
                assert_eq!(warnings.len(), 1, "{:?}", r.diagnostics());
                let loc = warnings[0].loc.as_ref().unwrap();
                assert_eq!(loc.line, 4);
                assert_eq!(loc.col as usize, column(&line, if version == "0.4" { "\\zzzz" } else { "@TEXT" }));
            }
        }
    }
}

#[test]
fn aliases_and_pin_labels_keep_their_parameter_location() {
    let cases = [
        (r#"@ALIAS("caption","text")"#, r#"(0,0) @TEXT(size=2,caption="\zzzz")"#, "@TEXT"),
        (r#"@ALIAS("caption","label")"#, r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1,caption="\zzzz") }"#, "@PIN"),
        ("", r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=[1,2],label="\zzzz") }"#, "@PIN"),
    ];
    for version in ["0.3", "0.4"] {
        for (aliases, body, keyword) in cases {
            let (r, line) = read(version, aliases, body, false);
            assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
            let warning = r.diagnostics().into_iter().find(|d| d.code == codes::MARKUP_UNKNOWN_COMMAND).unwrap();
            assert_eq!(
                warning.loc.as_ref().unwrap().col as usize,
                column(&line, if version == "0.4" { "\\zzzz" } else { keyword })
            );
        }
    }
}

#[test]
fn a_forwarded_text_points_to_its_parameter_use_and_keeps_the_application_origin() {
    let body = r#"$D = @DEFINE(roles=["r"],parameters=["caption"]) { ?r @TEXT(text=?caption) } }
* @RNL { #R1 @AS(caption="\zzzz") $D"#;
    let (r, line) = read("0.4", "", body, false);
    let warning = r.diagnostics().into_iter().find(|d| d.code == codes::MARKUP_UNKNOWN_COMMAND).unwrap();
    assert_eq!(warning.loc.as_ref().unwrap().line, 4);
    assert_eq!(warning.loc.as_ref().unwrap().col as usize, column(&line, "?caption)"));
    assert!(!warning.origin.is_empty());
}
