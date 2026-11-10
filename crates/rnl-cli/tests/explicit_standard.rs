//! Standard-package loading is an explicit draft setting (16.3).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn files(text: &str) -> MemoryProvider {
    let mut files = MemoryProvider { package_dirs: vec!["/pkg".into()], ..MemoryProvider::new() }.with("/t.sp", text);
    for version in ["0.2", "0.3", "0.4"] {
        files.insert(
            &format!("/pkg/{version}/rnl-std.rnl"),
            &format!(
                r#"@RNL(type="header") {{ @VERSION("{version}"); @PACKAGE(name="rnl-std", version="{version}") }}
@RNL {{ $D = @DEFINE(roles=["i"]) {{}} }}
"#
            ),
        );
    }
    files
}

fn source(version: &str, header: &str) -> String {
    format!("* explicit standard\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a b 1k\n* @RNL {{ #R1 @AS $D }}\n.end\n")
}

fn read(files: &MemoryProvider, options: Options) -> Interpretation {
    interpret_file(Path::new("/t.sp"), None, files, &options).unwrap()
}

fn errors(result: &Interpretation) -> usize {
    result.diagnostics().iter().filter(|d| d.severity == Severity::Error).count()
}

#[test]
fn a_draft_without_a_reference_uses_no_standard_by_default() {
    let result = read(&files(&source("0.4", "")), Options::solver());
    assert!(result.diagnostics().iter().any(|d| d.code == codes::REF_NOT_FOUND));
    assert!(result.problems[0].usages.is_empty());
}

#[test]
fn the_setting_adds_the_standard_package_for_the_main_draft_file() {
    let result = read(&files(&source("0.4", "")), Options { include_std: true, ..Options::solver() });
    assert_eq!(errors(&result), 0, "{:?}", result.diagnostics());
    assert_eq!(result.problems[0].usages.len(), 1);
    assert_eq!(result.included_standard.as_deref(), Some("0.4"));
}

#[test]
fn an_explicit_reference_works_with_both_settings_off() {
    let result = read(
        &files(&source("0.4", r#"@USE-PACKAGE(name="rnl-std", version="0.4")"#)),
        Options { std: false, ..Options::solver() },
    );
    assert_eq!(errors(&result), 0, "{:?}", result.diagnostics());
    assert_eq!(result.problems[0].usages.len(), 1);
    assert_eq!(result.included_standard, None);
}

#[test]
fn the_added_reference_is_saved_once_and_reads_without_the_setting() {
    for newline in ["\n", "\r\n"] {
        let text = source("0.4", "").replace('\n', newline);
        let result = read(&files(&text), Options { include_std: true, ..Options::solver() });
        let saved = rnl_runtime::writeback::with_standard_package(
            &result.doc,
            &text,
            result.included_standard.as_deref().unwrap(),
        );
        assert_eq!(saved.matches("@USE-PACKAGE").count(), 1);
        assert!(saved.contains(r#"@VERSION("0.4"); @USE-PACKAGE(name="rnl-std", version="0.4")"#));
        if newline == "\r\n" {
            assert!(!saved.replace("\r\n", "").contains('\n'));
        }
        let result = read(&files(&saved), Options { std: false, ..Options::solver() });
        assert_eq!(errors(&result), 0, "{:?}", result.diagnostics());
        assert_eq!(rnl_runtime::writeback::with_standard_package(&result.doc, &saved, "0.4"), saved);
    }
}

#[test]
fn solve_persists_the_setting_in_the_main_header() {
    let text = source("0.4", "");
    let options = rnl_cli::solve::SolveOptions { include_std: true, ..Default::default() };
    let run = rnl_cli::solve::solve(Path::new("/t.sp"), &files(&text), &options).unwrap();
    let saved = &run.files.iter().find(|f| f.path == "/t.sp").unwrap().text;
    assert_eq!(saved.matches("@USE-PACKAGE").count(), 1);
    let result = read(&files(saved), Options { std: false, ..Options::solver() });
    assert_eq!(errors(&result), 0, "{:?}", result.diagnostics());
}

#[test]
fn a_failed_explicit_header_reference_does_not_fall_back_to_the_setting() {
    let result = read(
        &files(&source("0.4", r#"@USE-PACKAGE(name="rnl-std", version="999")"#)),
        Options { include_std: true, ..Options::solver() },
    );
    assert!(errors(&result) > 0);
    assert!(result.problems[0].usages.is_empty());
    assert_eq!(result.included_standard, None);
}

#[test]
fn the_edit_writer_persists_its_reading_setting() {
    let request = serde_json::from_value(serde_json::json!({
        "language_version": "0.4", "dialect": "spice", "author": "editor", "file": "t.sp",
        "netlist": "* new\nR1 a b 1k\n.end\n", "ops": []
    }))
    .unwrap();
    let text = rnl_edit::write_with_options(&request, &files(""), &Options { include_std: true, ..Options::default() })
        .unwrap();
    assert_eq!(text.matches("@USE-PACKAGE").count(), 1);
    assert!(text.contains(r#"@USE-PACKAGE(name="rnl-std", version="0.4")"#));
}

#[test]
fn replace_records_persists_the_setting_only_when_writing_the_main_file() {
    let options = Options { include_std: true, ..Options::default() };
    let formats = Default::default();
    let reading = rnl_edit::replace::Reading { entry: "/t.sp", dialect: None, formats: &formats, options: &options };
    let result =
        rnl_edit::replace::replace_records(&reading, &files(&source("0.4", "")), "t.sp", "@RNL { #R1 @AS $D }")
            .unwrap();
    assert_eq!(result.text.matches("@USE-PACKAGE").count(), 1);
    assert_eq!(errors(&read(&files(&result.text), Options { std: false, ..Options::default() })), 0);

    let main = "* main\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include child.sp\n.end\n";
    let child = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a b\nR1 a b 1k\n* @RNL { #R1 @AS $D }\n.ends child\n";
    let result = rnl_edit::replace::replace_records(
        &reading,
        &files(main).with("/child.sp", child),
        "child",
        "@RNL { #R1 @AS $D }",
    )
    .unwrap();
    assert_eq!(result.path, "/child.sp");
    assert!(!result.text.contains("@USE-PACKAGE"));
}

#[test]
fn an_inline_drop_prevents_the_setting_from_adding_a_default() {
    let result = read(
        &files(&source("0.4", r#"@USE-PACKAGE(name="rnl-std") {}"#)),
        Options { include_std: true, ..Options::solver() },
    );
    assert!(result.diagnostics().iter().any(|d| d.code == codes::REF_NOT_FOUND));
    assert!(result.problems[0].usages.is_empty());
}

#[test]
fn released_versions_keep_their_implicit_standard_package() {
    for version in ["0.2", "0.3"] {
        let result = read(&files(&source(version, "")), Options::solver());
        assert_eq!(errors(&result), 0, "{version}: {:?}", result.diagnostics());
        assert_eq!(result.problems[0].usages.len(), 1);
    }
}

#[test]
fn a_draft_include_does_not_get_its_own_implicit_standard() {
    let main = "* mixed\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.include child.sp\n.end\n";
    let child = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a b\nR1 a b 1k\n* @RNL { #R1 @AS $D }\n.ends child\n";
    let result = read(&files(main).with("/child.sp", child), Options { include_std: true, ..Options::solver() });
    assert!(result
        .diagnostics()
        .iter()
        .any(|d| d.code == codes::REF_NOT_FOUND && d.loc.as_ref().is_some_and(|l| l.file.ends_with("child.sp"))));
}

#[test]
fn missing_standard_names_give_one_dynamic_package_hint_per_file() {
    let text = source("0.4", "").replace("#R1 @AS $D", "#R1 @AS $D; #R1 @AS $E; #R1 @AS $UNKNOWN");
    let mut provider = files(&text);
    provider.insert(
        "/pkg/0.4/rnl-std.rnl",
        r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="rnl-std", version="0.4") }
@RNL { $D = @DEFINE(roles=["i"]) {}; $E = @DEFINE(roles=["i"]) {}; @BAD-UNUSED-KEYWORD }
"#,
    );
    let result = read(&provider, Options::solver());
    let hints: Vec<_> =
        result.diagnostics().into_iter().filter(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED).collect();
    assert_eq!(hints.len(), 1, "{:?}", result.diagnostics());
    assert!(result.sink.render(hints[0]).contains("rnl-std"));
    assert_eq!(errors(&result), 3, "unused package errors must stay silent: {:?}", result.diagnostics());

    let result = read(&files(&source("0.4", "").replace("$D", "$UNKNOWN")), Options::solver());
    assert!(!result.diagnostics().iter().any(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED));
    let result = read(&files(&source("0.3", "")), Options { std: false, ..Options::solver() });
    assert!(!result.diagnostics().iter().any(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED));
}

#[test]
fn optional_package_hints_follow_the_available_collection_graph() {
    let mut provider = files(&source("0.4", "").replace("$D", "$OPTIONAL"));
    provider.insert("/pkg/0.4/rnl-std-full.rnl", r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="rnl-std-full", version="0.4"); @USE-PACKAGE(name="optional", version="0.4") }"#);
    provider.insert(
        "/pkg/0.4/optional.rnl",
        r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="optional", version="0.4") }
@RNL { $OPTIONAL = @DEFINE(roles=["i"]) {} }"#,
    );
    let result = read(&provider, Options::solver());
    let hint = result.diagnostics().into_iter().find(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED).unwrap();
    assert!(result.sink.render(hint).contains("package optional"));
}

#[test]
fn hint_lookup_uses_path_fallback_and_the_smallest_compatible_version() {
    let mut provider = files(&source("0.4", "").replace("$D", "$OPTIONAL"));
    provider.insert("/pkg/0.4/rnl-std-full.rnl", r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="rnl-std-full", version="0.4"); @USE-PACKAGE(name="optional", version="1.0", path="missing.rnl") }"#);
    for (version, name) in [("1.1", "OPTIONAL"), ("1.2", "NEWER")] {
        provider.insert(
            &format!("/pkg/{version}/optional.rnl"),
            &format!(
                r#"@RNL(type="header") {{ @VERSION("0.4"); @PACKAGE(name="optional", version="{version}") }}
@RNL {{ ${name} = @DEFINE(roles=["i"]) {{}} }}"#
            ),
        );
    }
    let result = read(&provider, Options::solver());
    assert!(result.diagnostics().iter().any(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED));
    // An existing path that is not a package fails lookup instead of using a different file.
    provider.insert("/pkg/0.4/missing.rnl", "@RNL { $OPTIONAL = @DEFINE {} }");
    let result = read(&provider, Options::solver());
    assert!(!result.diagnostics().iter().any(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED));
}

#[test]
fn missing_package_advice_is_once_for_each_source_file() {
    let main = source("0.4", "").replace(".end", ".include child.sp\n.end");
    let child = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a b\nR1 a b 1k\n* @RNL { #R1 @AS $D; #R1 @AS $D }\n.ends child\n";
    let result = read(&files(&main).with("/child.sp", child), Options::solver());
    let mut sources: Vec<_> = result
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == codes::STANDARD_PACKAGE_NOT_USED)
        .map(|d| d.loc.as_ref().unwrap().file.as_str())
        .collect();
    sources.sort_unstable();
    assert_eq!(sources, ["child.sp", "t.sp"]);
}

#[test]
fn the_setting_pins_the_newest_series_written_in_the_files_language() {
    let text = source("0.4", "");
    let mut provider = files(&text);
    provider.insert(
        "/pkg/1.7/rnl-std.rnl",
        r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="rnl-std", version="1.7") }
@RNL { $D = @DEFINE(roles=["i"]) {} }"#,
    );
    let result = read(&provider, Options { include_std: true, ..Options::solver() });
    assert_eq!(errors(&result), 0, "{:?}", result.diagnostics());
    assert_eq!(result.included_standard.as_deref(), Some("1.7"));
    let saved = rnl_runtime::writeback::with_standard_package(&result.doc, &text, "1.7");
    provider.insert("/t.sp", &saved);
    assert_eq!(errors(&read(&provider, Options { std: false, ..Options::solver() })), 0);
}

#[test]
fn command_line_setting_reaches_check_solve_and_both_edit_commands() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let folder = std::env::temp_dir().join(format!(
        "rnl-explicit-standard-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("t.sp"), source("0.4", "")).unwrap();
    fs::write(folder.join("rnl-std.rnl"), "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"rnl-std\", version=\"0.4\") }\n@RNL { $D = @DEFINE(roles=[\"i\"]) {} }").unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rnl"))
            .current_dir(&folder)
            .args(args)
            .args(["--package-dir", "."])
            .output()
            .unwrap()
    };
    for (flags, expected) in [
        (vec![], 1),
        (vec!["--include-std"], 0),
        (vec!["--include-std", "--no-std"], 1),
        (vec!["--no-std", "--include-std"], 0),
    ] {
        let mut args = vec!["check", "t.sp"];
        args.extend(flags);
        let result = run(&args);
        assert_eq!(
            result.status.code(),
            Some(expected),
            "{args:?}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let result = run(&["solve", "t.sp", "--include-std", "-o", "solved"]);
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(fs::read_to_string(folder.join("solved/t.sp")).unwrap().contains("@USE-PACKAGE"));

    let request = serde_json::json!({ "language_version": "0.4", "dialect": "spice", "author": "test", "file": "t.sp", "netlist": "* new\nR1 a b 1k\n.end\n", "ops": [] });
    fs::write(folder.join("write.json"), request.to_string()).unwrap();
    let result = run(&["write", "write.json", "-o", "written.sp", "--include-std"]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(fs::read_to_string(folder.join("written.sp")).unwrap().contains("@USE-PACKAGE"));
    fs::write(folder.join("records.rnl"), "@RNL { #R1 @AS $D }").unwrap();
    let result =
        run(&["replace-records", "t.sp", "--level", "t.sp", "--records", "records.rnl", "--in-place", "--include-std"]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(fs::read_to_string(folder.join("t.sp")).unwrap().contains("@USE-PACKAGE"));
    assert!(run(&["check", "t.sp", "--no-std"]).status.success());
    fs::remove_dir_all(folder).unwrap();
}
