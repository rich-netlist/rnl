//! Package versions are unsigned decimal pairs, independently of the host language (16.2).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn package(language: &str, version: &str, references: &str, name: &str) -> String {
    format!(
        r#"@RNL(type="header") {{ @PACKAGE(name="p",version="{version}"); @VERSION("{language}"); {references} }}
@RNL {{ ${name} = @DEFINE(roles=["i"]) {{}} }}"#
    )
}

fn source(language: &str, reference: &str, body: &str) -> String {
    format!("* versions\n* @RNL(type=\"header\") {{ @VERSION(\"{language}\"); {reference} }}\nR1 a 0 1k\n* @RNL {{ {body} }}\n.end\n")
}

fn read(files: &MemoryProvider, path: &str) -> Interpretation {
    interpret_file(Path::new(path), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn clean(result: &Interpretation) {
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
}

#[test]
fn malformed_versions_fail_before_lookup_or_inline_expansion() {
    for version in ["", "1", "1.2.3", "-1.2", "1.-2", "+1.2", "1.+2", " 1.2", "1.2 ", "１.２", ".2", "1."] {
        for keyword in ["USE-PACKAGE", "USE-ALIASES"] {
            for inline in [false, true] {
                if inline && keyword == "USE-ALIASES" {
                    continue;
                }
                let body = if inline { "{ $BAD = @DEFINE(roles=[\"i\"]) {} }" } else { "" };
                let reference = format!("@{keyword}(name=\"p\",version=\"{version}\") {body}");
                let mut files = MemoryProvider::new().with("t.sp", &source("0.4", &reference, "#R1 @AS $BAD"));
                files.package_dirs.push("pkg".into());
                // Searching this candidate would issue a filename mismatch warning.
                files.insert("pkg/p.rnl", "not a package");
                let result = read(&files, "t.sp");
                assert_eq!(
                    result.diagnostics().iter().filter(|d| d.code == codes::PACKAGE_VERSION_MALFORMED).count(),
                    1,
                    "{version:?} {keyword} {inline}: {:?}",
                    result.diagnostics()
                );
                assert!(!result.diagnostics().iter().any(|d| matches!(
                    d.code,
                    codes::PACKAGE_NOT_FOUND | codes::PACKAGE_FILE_NAME_MISMATCH | codes::PACKAGE_NO_VERSION_NEWEST
                )));
                assert!(result.problems[0].usages.is_empty());
            }
        }
    }
}

#[test]
fn malformed_package_declarations_are_not_packages() {
    for version in ["nonsense", "1.-2", "1", "1.2.3"] {
        let text = package("0.4", version, "", "D");
        assert!(rnl_runtime::interp::unit::scan_package_decl(&text).is_none());
        let result = read(&MemoryProvider::new().with("p.rnl", &text), "p.rnl");
        assert!(
            result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_VERSION_MALFORMED),
            "{:?}",
            result.diagnostics()
        );
    }
}

#[test]
fn an_existing_nonpackage_path_never_falls_back() {
    for text in ["not a package".to_string(), package("0.4", "bad", "", "D")] {
        let reference = r#"@USE-PACKAGE(name="p",version="1.0",path="bad.rnl")"#;
        let mut files = MemoryProvider::new().with("t.sp", &source("0.4", reference, "#R1 @AS $D"));
        files.package_dirs.push("pkg".into());
        files.insert("bad.rnl", &text);
        files.insert("pkg/p.rnl", &package("0.4", "1.0", "", "D"));
        let result = read(&files, "t.sp");
        assert!(result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_PATH_NOT_PACKAGE));
        assert!(!result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_PATH_NOT_FOUND));
        assert!(result.problems[0].usages.is_empty());
    }
}

#[test]
fn missing_and_mismatched_paths_still_fall_back() {
    for text in [None, Some(package("0.4", "2.0", "", "OTHER"))] {
        let mut files = MemoryProvider::new()
            .with("t.sp", &source("0.4", r#"@USE-PACKAGE(name="p",version="1.0",path="bad.rnl")"#, "#R1 @AS $D"));
        files.package_dirs.push("pkg".into());
        files.insert("pkg/p.rnl", &package("0.4", "1.0", "", "D"));
        if let Some(text) = text {
            files.insert("bad.rnl", &text);
        }
        let result = read(&files, "t.sp");
        clean(&result);
        assert_eq!(result.problems[0].usages.len(), 1);
        assert!(result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_PATH_NOT_FOUND));
    }
}

#[test]
fn decimal_versions_have_no_machine_integer_limit() {
    let major = "9999999999999999999999999999999999999999";
    let minor = "8888888888888888888888888888888888888888";
    let mut files = MemoryProvider::new();
    files.package_dirs.push("pkg".into());
    files.insert("pkg/low.rnl", &package("0.4", &format!("{major}.1"), "", "LOW"));
    files.insert("pkg/mid.rnl", &package("0.4", &format!("{major}.20"), "", "MID"));
    files.insert("pkg/high.rnl", &package("0.4", &format!("{major}.{minor}"), "", "HIGH"));
    for (version, name) in [(Some("2"), "MID"), (Some(minor), "HIGH"), (None, "HIGH")] {
        let version = version.map_or(String::new(), |v| format!(",version=\"{major}.{v}\""));
        files
            .insert("t.sp", &source("0.4", &format!("@USE-PACKAGE(name=\"p\"{version})"), &format!("#R1 @AS ${name}")));
        let result = read(&files, "t.sp");
        clean(&result);
        assert_eq!(result.problems[0].usages.len(), 1);
    }
}

#[test]
fn legacy_version_and_path_handling_stays_unchanged() {
    for language in ["0.2", "0.3"] {
        for version in ["1", "+1.2", "1.-2"] {
            let mut files = MemoryProvider::new().with(
                "t.sp",
                &source(
                    language,
                    &format!("@USE-PACKAGE(name=\"p\",version=\"{version}\",path=\"bad.rnl\")"),
                    "#R1 @AS $D",
                ),
            );
            files.package_dirs.push("pkg".into());
            files.insert("bad.rnl", "not a package");
            files.insert("pkg/p.rnl", &package(language, version, "", "D"));
            let result = read(&files, "t.sp");
            clean(&result);
            assert_eq!(result.problems[0].usages.len(), 1);
        }
    }
}

#[test]
fn repeated_package_declarations_keep_the_first_in_the_draft() {
    for language in ["0.2", "0.3", "0.4"] {
        let text = package(language, "1.0", r#"@PACKAGE(name="q",version="2.0")"#, "D");
        let expected = if language == "0.4" { ("p", "1.0") } else { ("q", "2.0") };
        assert_eq!(rnl_runtime::interp::unit::scan_package_decl(&text), Some((expected.0.into(), expected.1.into())));
        let result = read(&MemoryProvider::new().with("p.rnl", &text), "p.rnl");
        assert_eq!(result.checking_package, expected.0);
        assert_eq!(
            result.diagnostics().iter().filter(|d| d.code == codes::PACKAGE_DECLARED_TWICE).count(),
            usize::from(language == "0.4")
        );
    }
    let text = package("0.4", "bad", r#"@PACKAGE(name="p",version="1.0")"#, "D");
    assert!(rnl_runtime::interp::unit::scan_package_decl(&text).is_none());
    let result = read(&MemoryProvider::new().with("p.rnl", &text), "p.rnl");
    assert!(result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_VERSION_MALFORMED));
}

#[test]
fn malformed_references_inside_packages_keep_independent_names() {
    let text = package("0.4", "1.0", r#"@USE-PACKAGE(name="missing",version="-1.0");"#, "D");
    let files = MemoryProvider::new()
        .with("t.sp", &source("0.4", r#"@USE-PACKAGE(name="p",version="1.0",path="p.rnl")"#, "#R1 @AS $D"))
        .with("p.rnl", &text);
    let result = read(&files, "t.sp");
    let error = result.diagnostics().into_iter().find(|d| d.code == codes::PACKAGE_VERSION_MALFORMED).unwrap();
    assert_eq!(error.loc.as_ref().unwrap().file, "p.rnl");
    assert_eq!(error.in_package, "p");
    assert_eq!(result.problems[0].usages.len(), 1);
}

#[test]
fn transitive_versions_use_the_newest_decimal_pair_of_the_same_major() {
    let major = "9999999999999999999999999999999999";
    let mut files = MemoryProvider::new().with(
        "t.sp",
        &source(
            "0.4",
            r#"@USE-PACKAGE(name="a",version="1.0"); @USE-PACKAGE(name="b",version="1.0")"#,
            "#R1 @AS $HIGH",
        ),
    );
    files.package_dirs.push("pkg".into());
    for (name, minor, definition) in [("a", "1", "LOW"), ("b", "999999999999999999999999", "HIGH")] {
        files.insert(&format!("pkg/{name}.rnl"), &format!(r#"@RNL(type="header") {{ @VERSION("0.4"); @PACKAGE(name="{name}",version="1.0"); @USE-PACKAGE(name="p",version="{major}.{minor}") }}"#));
        files.insert(&format!("pkg/p-{name}.rnl"), &package("0.4", &format!("{major}.{minor}"), "", definition));
    }
    let result = read(&files, "t.sp");
    clean(&result);
    assert_eq!(result.problems[0].usages.len(), 1);
    assert!(result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_REFERENCE_OLDER_VERSION));
}

#[test]
fn zero_series_do_not_substitute_and_leading_zeroes_are_numeric() {
    let mut files = MemoryProvider::new();
    files.package_dirs.push("pkg".into());
    files.insert("pkg/p.rnl", &package("0.4", "00.04", "", "D"));
    for (wanted, found) in [("0.4", true), ("0.3", false), ("0.5", false)] {
        files.insert("t.sp", &source("0.4", &format!("@USE-PACKAGE(name=\"p\",version=\"{wanted}\")"), "#R1 @AS $D"));
        let result = read(&files, "t.sp");
        assert_eq!(result.problems[0].usages.len(), usize::from(found));
        if found {
            clean(&result);
        } else {
            assert!(result.diagnostics().iter().any(|d| d.code == codes::PACKAGE_NOT_FOUND));
        }
    }
}

#[test]
fn equivalent_decimal_spellings_share_a_cached_package_reading() {
    let files = MemoryProvider::new()
        .with("t.sp", &source("0.4", r#"@USE-PACKAGE(name="p",version="01.0",path="first.rnl"); @USE-PACKAGE(name="p",version="1.00",path="second.rnl")"#, "#R1 @AS $D"))
        .with("first.rnl", &package("0.4", "01.0", "", "D"))
        .with("second.rnl", &package("0.4", "1.00", "", "OTHER"));
    let result = read(&files, "t.sp");
    clean(&result);
    assert_eq!(result.problems[0].usages.len(), 1);
}

#[test]
fn the_tool_standard_setting_uses_the_newest_unbounded_version() {
    let huge = "9999999999999999999999999999999999.0";
    let mut files = MemoryProvider::new().with("t.sp", &source("0.4", "", "#R1 @AS $HIGH"));
    files.package_dirs.push("pkg".into());
    for (name, version, definition) in [("low", "1.0", "LOW"), ("high", huge, "HIGH")] {
        files.insert(
            &format!("pkg/{name}.rnl"),
            &package("0.4", version, "", definition).replace("name=\"p\"", "name=\"rnl-std-all\""),
        );
    }
    let result =
        interpret_file(Path::new("t.sp"), None, &files, &Options { include_std: true, ..Options::solver() }).unwrap();
    clean(&result);
    assert_eq!(result.included_standard.as_deref(), Some(huge));
    assert_eq!(result.problems[0].usages.len(), 1);
}
