//! Duplicate slot labels identify the repeated role without changing released diagnostics.
use std::path::Path;

use rnl_runtime::diag::codes;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, header: &str, body: &str) -> (Interpretation, String) {
    let line = format!("* @RNL {{ {body} }}");
    let text=format!("* role diagnostics\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a b 1k\nR2 a b 2k\n{line}\n");
    let r = rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    (r, line)
}

#[test]
fn repeated_written_labels_name_the_role_and_point_to_the_second_object() {
    for version in ["0.2", "0.3", "0.4"] {
        let (r, line) = read(version, "", r#"#R1(from) #R2(from) @DISTANCE(direction="left-to-right",exact=1)"#);
        let matching: Vec<_> =
            r.diagnostics().into_iter().filter(|d| d.code == codes::KEYWORD_POSITION_LABELLED_TWICE).collect();
        assert_eq!(matching.len(), 1, "{:?}", r.diagnostics());
        let d = matching[0];
        assert_eq!(d.loc.as_ref().unwrap().line, 5);
        if version == "0.4" {
            assert_eq!(d.message, "role from is labelled more than once (4.2 rule 6)");
            assert_eq!(d.loc.as_ref().unwrap().col as usize, line.find("#R2").unwrap() + 1);
        } else {
            assert_eq!(d.message, "a role is labelled only once (4.2 rule 6)");
            assert_eq!(d.loc.as_ref().unwrap().col as usize, line.find("#R1").unwrap() + 1);
        }
    }
}

#[test]
fn aliases_report_the_canonical_role_at_the_repeated_object() {
    for version in ["0.3", "0.4"] {
        let (r, line) = read(
            version,
            r#"@ALIAS("origin","from")"#,
            r#"#R1(from) #R2(origin) @DISTANCE(direction="left-to-right",exact=1)"#,
        );
        let matching: Vec<_> =
            r.diagnostics().into_iter().filter(|d| d.code == codes::KEYWORD_POSITION_LABELLED_TWICE).collect();
        assert_eq!(matching.len(), 1, "{:?}", r.diagnostics());
        assert_eq!(
            matching[0].message,
            if version == "0.4" {
                "role from is labelled more than once (4.2 rule 6)"
            } else {
                "a role is labelled only once (4.2 rule 6)"
            }
        );
        assert_eq!(matching[0].loc.as_ref().unwrap().col as usize, line.find("#R2").unwrap() + 1);
    }
}

#[test]
fn fixed_aliased_role_duplicates_in_a_definition_are_reported_once() {
    for calls in ["", "(0,0) (1,1) @AS $D; (2,2) (3,3) @AS $D"] {
        let body=format!("$D = @DEFINE(roles=[\"p\",\"q\"]) {{ ?p(from) ?q(origin) @DISTANCE(direction=\"left-to-right\",exact=1) }}; {calls}");
        let (r, _) = read("0.4", r#"@ALIAS("origin","from")"#, &body);
        let matching: Vec<_> =
            r.diagnostics().into_iter().filter(|d| d.code == codes::KEYWORD_POSITION_LABELLED_TWICE).collect();
        assert_eq!(matching.len(), 1, "{:?}", r.diagnostics());
        assert_eq!(matching[0].message, "role from is labelled more than once (4.2 rule 6)");
        assert!(matching[0].origin.is_empty());
    }
}
