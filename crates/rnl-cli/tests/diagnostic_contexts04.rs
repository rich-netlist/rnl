//! Diagnostics identify the object, count and package reference that need attention.
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn source(header: &str, body: &str) -> String {
    format!("* context\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); {header} }}\nR1 a b 1k\n* @RNL {{ {body} }}\n")
}

fn read(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn body(body: &str) -> Interpretation {
    read(&MemoryProvider::new().with("main.sp", &source("", body)))
}

fn messages(r: &Interpretation, code: Code) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.code == code).map(|d| d.message.clone()).collect()
}

#[test]
fn graphic_counts_state_the_expected_and_actual_points() {
    for (keyword, points, expected) in [
        ("LINE", "(0,0)", "at least 2 points, but 1 point is"),
        ("ARC", "{(0,0) (1,1)}", "3 points, but 2 points are"),
        ("BEZIER", "{(0,0) (1,1) (2,1) (3,0) (4,0)}", "4 points plus 3 for each additional segment, but 5 points are"),
        ("RECTANGLE", "(0,0)", "2 points, but 1 point is"),
        ("CIRCLE", "{(0,0) (1,1) (2,2)}", "2 points, but 3 points are"),
        ("ELLIPSE", "{(0,0) (1,1) (2,2)}", "2 points, but 3 points are"),
    ] {
        let r = body(&format!("{points} @{keyword}; (5,5) @CIRCLE(radius=1)"));
        assert_eq!(
            messages(&r, codes::KEYWORD_GRAPHIC_POINT_COUNT),
            [format!("@{keyword} needs {expected} written (9.2.5)")]
        );
        assert_eq!(r.model.levels[0].shapes.len(), 1);
    }
}

#[test]
fn unused_and_reused_definitions_share_the_point_count_message() {
    for calls in ["", "$D @ATTACH; $D @ATTACH"] {
        let r = body(&format!("$D = @DEFINE(roles=[]) {{ (0,0) @LINE; (1,1) @CIRCLE(radius=1) }}; {calls}"));
        assert_eq!(
            messages(&r, codes::KEYWORD_GRAPHIC_POINT_COUNT),
            ["@LINE needs at least 2 points, but 1 point is written (9.2.5)"]
        );
        let d = r.diagnostics().into_iter().find(|d| d.code == codes::KEYWORD_GRAPHIC_POINT_COUNT).unwrap();
        assert!(d.origin.is_empty());
        assert_eq!(d.loc.as_ref().unwrap().line, 4);
    }
}

#[test]
fn a_role_filter_names_the_scope_and_current_section() {
    let r = body("#R1 @SELECT-INSTANCE(role=\"left\")");
    assert_eq!(
        messages(&r, codes::SELECT_ROLE_WITHOUT_USAGE_SCOPE),
        ["role= applies only to a usage scope; #R1 is not a usage (10.1)"]
    );
}

fn package(name: &str, number: &str, aliases: bool) -> String {
    let header = if aliases { "@ALIAS(\"@locate\",\"@PLACE\")" } else { "" };
    let contents = if aliases { "" } else { "@RNL { $D = @DEFINE(roles=[\"r\"]) {} }" };
    format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"{name}\",version=\"{number}\"); {header} }}\n{contents}\n")
}

#[test]
fn package_path_warnings_explain_the_mismatch_without_preventing_fallback() {
    for aliases in [false, true] {
        let keyword = if aliases { "USE-ALIASES" } else { "USE-PACKAGE" };
        let kind = if aliases { "alias package" } else { "package" };
        for supplied in [None, Some(("other", "1.0")), Some(("wanted", "2.0"))] {
            let reference = format!("@{keyword}(name=\"wanted\",version=\"1.0\",path=\"bad.rnl\")");
            let application = if aliases { "#R1 @locate(1,2)" } else { "#R1 @AS $D" };
            let mut files = MemoryProvider::new().with("main.sp", &source(&reference, application));
            files.package_dirs.push("pkg".into());
            files.insert("pkg/wanted.rnl", &package("wanted", "1.0", aliases));
            if let Some((name, number)) = supplied {
                files.insert("bad.rnl", &package(name, number, aliases));
            }
            let r = read(&files);
            assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
            let diagnostics: Vec<_> =
                r.diagnostics().into_iter().filter(|d| d.code == codes::PACKAGE_PATH_NOT_FOUND).collect();
            assert_eq!(diagnostics.len(), 1);
            let message = &diagnostics[0].message;
            let section = if aliases { "16.5" } else { "16.2" };
            let reason = match supplied {
                None => format!("path=\"bad.rnl\" for {kind} wanted cannot be read"),
                Some((name, number)) => {
                    format!("path=\"bad.rnl\" declares {kind} {name} {number}, but this reference requests wanted 1.0")
                }
            };
            assert_eq!(message, &format!("{reason}; looking up {kind} wanted by name ({section})"));
            assert_eq!(diagnostics[0].loc.as_ref().unwrap().line, 2);
            if !aliases {
                assert_eq!(r.problems[0].usages.len(), 1);
            }
        }
    }
}
