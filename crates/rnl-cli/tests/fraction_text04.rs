//! Malformed text fractions remain visible and valid text in draft 0.4 (13.6).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::markup::{self, plain, Run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn source(version: &str, markup: &str) -> String {
    format!("* fractions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ (0,0) @TEXT(\"{markup}\"); (0,4) @TEXT(\"after\") }}\n")
}
fn read(files: &MemoryProvider) -> Interpretation {
    let r = interpret_file(Path::new("main.sp"), None, files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

#[test]
fn a_missing_group_warns_and_preserves_the_written_command() {
    for text in [r"\frac", r"\frac{a}", r"\frac{a}tail", r"\frac{a}{b", r"\frac{\textbf{x}}", r"\frac{a"] {
        let r = read(&MemoryProvider::new().with("main.sp", &source("0.4", text)));
        assert_eq!(
            r.diagnostics().iter().filter(|d| d.code == codes::MARKUP_FRAC_BRACES).count(),
            1,
            "{text}: {:?}",
            r.diagnostics()
        );
        let texts = &r.model.levels[0].texts;
        assert_eq!(plain(&texts[0].lines[0]), text);
        assert!(texts[0].lines[0].iter().all(|run| matches!(run, Run::Text { .. })));
        assert_eq!(plain(&texts[1].lines[0]), "after");
    }
}

#[test]
fn complete_groups_accept_spacing_nesting_and_literal_attributes() {
    for text in [r"\frac{a}{b}", r"\frac {a}{b}", r"\frac{a} {b}", "\\frac\t{a}\t{b}"] {
        let r = read(&MemoryProvider::new().with("main.sp", &source("0.4", text)));
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Warning), "{:?}", r.diagnostics());
        let Run::Frac { num, den, .. } = &r.model.levels[0].texts[0].lines[0][0] else { panic!("{text}") };
        assert_eq!((plain(num), plain(den)), ("a".into(), "b".into()));
    }
    let text = markup::expand(r"\frac{a{b}+\frac{x}{y}+${value}}{z}", |_| r"}\frac{bad}".into());
    let runs = markup::parse_with_name_formats(&text, true);
    let Run::Frac { num, den, .. } = &runs[0][0] else { panic!("{:?}", runs[0]) };
    assert_eq!(plain(num), r"a{b}+x/y+}\frac{bad}");
    assert_eq!(plain(den), "z");
    assert!(num.iter().any(|run| matches!(run, Run::Frac { .. })));
}

#[test]
fn math_fraction_validation_stays_in_the_formula_parser() {
    let r = read(&MemoryProvider::new().with("main.sp", &source("0.4", r"\(\frac{a}\)")));
    assert!(r.diagnostics().iter().any(|d| d.code == codes::MARKUP_UNKNOWN_COMMAND));
    assert!(!r.diagnostics().iter().any(|d| d.code == codes::MARKUP_FRAC_BRACES));
    assert_eq!(plain(&r.model.levels[0].texts[0].lines[0]), r"\frac{a}");
}

#[test]
fn included_pin_labels_warn_in_their_defining_file() {
    let top = "* fractions\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include child.sp\nX1 a child\n";
    let included = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a\nR1 a 0 1k\n* @RNL { $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1,label=\"\\frac{a}\") }; #R1 @SET-SYMBOL $S => @PLACE(0,0) }\n.ends child\n";
    let r = read(&MemoryProvider::new().with("main.sp", top).with("child.sp", included));
    let diagnostics: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::MARKUP_FRAC_BRACES).collect();
    assert!(!diagnostics.is_empty(), "{:?}", r.diagnostics());
    assert!(diagnostics[0].loc.as_ref().unwrap().file.ends_with("child.sp"));
}
