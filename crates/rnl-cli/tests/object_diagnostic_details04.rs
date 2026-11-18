//! Invalid symbol pin and style targets name the supplied object.
use rnl_runtime::diag::codes;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(body: &str) -> Interpretation {
    let source = format!(
        "* target diagnostics\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n"
    );
    rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

#[test]
fn invalid_pin_targets_identify_the_supplied_object() {
    for (body, label) in [
        ("$T = @DEFINE-SYMBOL {}; $S = @DEFINE-SYMBOL { $T @PIN(terminal=1) }", "$T"),
        ("$T = @DEFINE-STYLE {}; $S = @DEFINE-SYMBOL { $T @PIN(terminal=1) }", "$T"),
        ("$S = @DEFINE-SYMBOL { {(0,0) (1,1)} @PIN(terminal=1) }", "a set"),
    ] {
        let r = read(body);
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::KEYWORD_PIN_NOT_A_POINT).collect();
        assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
        assert_eq!(errors[0].message, format!("PIN takes one point; {label} is not a point (9.2.4)"));
        assert_eq!(errors[0].loc.as_ref().unwrap().line, 4);
    }
}

#[test]
fn invalid_style_members_identify_the_supplied_object() {
    for (body, label) in
        [("$BAD = @DEFINE-STYLE { (1,2) }", "(1, 2)"), ("$S = @DEFINE-SYMBOL {}; $BAD = @DEFINE-STYLE { $S }", "$S")]
    {
        let r = read(body);
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::STYLE_BODY_NOT_STYLE).collect();
        assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
        assert_eq!(
            errors[0].message,
            format!("{label} is not a style; the body of DEFINE-STYLE only references styles (9.3)")
        );
        assert_eq!(errors[0].loc.as_ref().unwrap().line, 4);
    }
}

#[test]
fn orientation_type_errors_cite_the_orientation_relation_rule() {
    let r = read(r#"(0,0) (1,1) @ORIENT-RELATION("R0")"#);
    let errors: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.code == codes::CONSTRAINT_MEMBER_NOT_LOCATABLE).collect();
    assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
    assert!(errors[0].message.contains("(0, 0) has no orientation"));
    assert!(errors[0].message.ends_with("(14.9)"));
}
