//! Diagnostics identify the object, count and package reference that need attention.
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn source(version: &str, header: &str, body: &str) -> String {
    format!(
        "* context\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a b 1k\n* @RNL {{ {body} }}\n"
    )
}

fn read(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn body(version: &str, body: &str) -> Interpretation {
    read(&MemoryProvider::new().with("main.sp", &source(version, "", body)))
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
        for version in ["0.2", "0.3", "0.4"] {
            let r = body(version, &format!("{points} @{keyword}; (5,5) @CIRCLE(radius=1)"));
            let expected = if version == "0.4" {
                format!("@{keyword} needs {expected} written (9.2.5)")
            } else {
                format!("@{keyword} has the wrong number of points (9.2.5)")
            };
            assert_eq!(messages(&r, codes::KEYWORD_GRAPHIC_POINT_COUNT), [expected]);
            assert_eq!(r.model.levels[0].shapes.len(), 1);
        }
    }
}

#[test]
fn unused_and_reused_definitions_share_the_point_count_message() {
    for calls in ["", "$D @ATTACH; $D @ATTACH"] {
        let r = body("0.4", &format!("$D = @DEFINE(roles=[]) {{ (0,0) @LINE; (1,1) @CIRCLE(radius=1) }}; {calls}"));
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
    for version in ["0.3", "0.4"] {
        let r = body(version, "#R1 @SELECT-INSTANCE(role=\"left\")");
        let expected = if version == "0.4" {
            "role= applies only to a usage scope; #R1 is not a usage (10.1)"
        } else {
            "role= means something only when the scope is a usage (K3.12)"
        };
        assert_eq!(messages(&r, codes::SELECT_ROLE_WITHOUT_USAGE_SCOPE), [expected]);
    }
}

fn package(version: &str, name: &str, number: &str, aliases: bool) -> String {
    let header = if aliases { "@ALIAS(\"@locate\",\"@PLACE\")" } else { "" };
    let contents = if aliases { "" } else { "@RNL { $D = @DEFINE(roles=[\"r\"]) {} }" };
    format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"{name}\",version=\"{number}\"); {header} }}\n{contents}\n")
}

#[test]
fn package_path_warnings_explain_the_mismatch_without_preventing_fallback() {
    for version in ["0.3", "0.4"] {
        for aliases in [false, true] {
            let keyword = if aliases { "USE-ALIASES" } else { "USE-PACKAGE" };
            let kind = if aliases { "alias package" } else { "package" };
            for supplied in [None, Some(("other", "1.0")), Some(("wanted", "2.0"))] {
                let reference = format!("@{keyword}(name=\"wanted\",version=\"1.0\",path=\"bad.rnl\")");
                let application = if aliases { "#R1 @locate(1,2)" } else { "#R1 @AS $D" };
                let mut files = MemoryProvider::new().with("main.sp", &source(version, &reference, application));
                files.package_dirs.push("pkg".into());
                files.insert("pkg/wanted.rnl", &package(version, "wanted", "1.0", aliases));
                if let Some((name, number)) = supplied {
                    files.insert("bad.rnl", &package(version, name, number, aliases));
                }
                let r = read(&files);
                assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
                let diagnostics: Vec<_> =
                    r.diagnostics().into_iter().filter(|d| d.code == codes::PACKAGE_PATH_NOT_FOUND).collect();
                assert_eq!(diagnostics.len(), 1);
                let message = &diagnostics[0].message;
                let section = if aliases { "16.5" } else { "16.2" };
                if version == "0.4" {
                    let reason = match supplied {
                        None => format!("path=\"bad.rnl\" for {kind} wanted cannot be read"),
                        Some((name, number)) => format!(
                            "path=\"bad.rnl\" declares {kind} {name} {number}, but this reference requests wanted 1.0"
                        ),
                    };
                    assert_eq!(message, &format!("{reason}; looking up {kind} wanted by name ({section})"));
                } else {
                    assert_eq!(message, &format!("the file named by path= does not exist or its @PACKAGE does not match; looking the package up by name ({section})"));
                }
                assert_eq!(diagnostics[0].loc.as_ref().unwrap().line, 2);
                if !aliases {
                    assert_eq!(r.problems[0].usages.len(), 1);
                }
            }
        }
    }
}
