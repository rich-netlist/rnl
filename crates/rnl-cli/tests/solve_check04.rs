//! `rnl solve --check`: the solver's settings of each level, with where each comes from, checked
//! without solving or writing anything (14.10).
use rnl_cli::solve::{check_settings, Checked, CheckedLevel, SolveOptions, ToolSettings};
use rnl_runtime::source::MemoryProvider;
use std::path::Path;

const MAIN: &str = r#"* settings from everywhere
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="fixture",version="1.0"); @SOLVER-CONFIG(spacing_x=2, beam=8, seconds=3, tolerance=-1) }
.subckt amp a b
R1 a b 1k
* @RNL { @SOLVER-CONFIG(beam=6, layout="tree", grid="x") }
.ends
X1 n1 n2 amp
R2 n2 0 1k
.end
"#;
const PACKAGE: &str = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="fixture",version="1.0"); @SOLVER-CONFIG(beam=3, grid=2) }
"#;

fn files(main: &str) -> MemoryProvider {
    let mut files = MemoryProvider::new().with("main.sp", main).with("packages/fixture.rnl", PACKAGE);
    files.package_dirs.push("packages".into());
    files
}

fn check(main: &str, tool: &[ToolSettings]) -> Checked {
    let options = SolveOptions { std: false, ..SolveOptions::default() };
    check_settings(Some(Path::new("main.sp")), &files(main), &options, tool).unwrap()
}

fn level<'a>(c: &'a Checked, title: &str) -> &'a CheckedLevel {
    c.levels.iter().find(|l| l.title.as_deref() == Some(title)).unwrap()
}

/// Each setting's value and where it comes from, as `key=value (from)`.
fn setting(l: &CheckedLevel, key: &str) -> String {
    let s = &l.settings[key];
    format!("{key}={} ({})", s.value, s.from)
}

fn keyed(problems: &[rnl_cli::solve::CheckedProblem]) -> Vec<String> {
    problems.iter().map(|p| format!("{}: {}", p.key.as_deref().unwrap_or("-"), p.message)).collect()
}

#[test]
fn each_setting_says_where_it_comes_from_in_the_order_of_14_10() {
    let c = check(MAIN, &[ToolSettings::Config("spacing_y=4".into())]);
    assert!(c.ok, "{c:?}");
    let top = level(&c, "main.sp");
    assert_eq!(setting(top, "astar_expansions"), "astar_expansions=200000 (default)");
    assert_eq!(setting(top, "grid"), "grid=2 (package:fixture)");
    // the header comes after the package
    assert_eq!(setting(top, "beam"), "beam=8 (header)");
    assert_eq!(setting(top, "spacing_x"), "spacing_x=2 (header)");
    assert_eq!(setting(top, "spacing_y"), "spacing_y=4 (command-line)");
    // where a setting of the file or a package is written
    assert_eq!(top.settings["spacing_x"].at.as_deref(), Some("main.sp:2"));
    assert_eq!(top.settings["grid"].at.as_deref(), Some("fixture.rnl:1"));
    assert_eq!(top.settings["spacing_y"].at, None);
    // the level after the header
    let amp = level(&c, "amp");
    assert_eq!(setting(amp, "beam"), "beam=6 (level)");
    assert_eq!(setting(amp, "layout"), "layout=\"tree\" (level)");
    assert_eq!(setting(top, "layout"), "layout=\"lines\" (default)");
}

#[test]
fn a_bad_value_of_the_file_is_a_warning_and_the_value_before_it_stands() {
    let c = check(MAIN, &[]);
    let amp = level(&c, "amp");
    let warnings = keyed(&amp.warnings);
    let grid =
        "grid: grid must be a finite number; grid=\"x\" at main.sp:5 is ignored, and grid=2 at fixture.rnl:1 stands";
    assert!(warnings.contains(&grid.to_string()), "{warnings:?}");
    assert!(warnings.contains(&"seconds: seconds is a tool-only run setting and is ignored in the file".to_string()));
    let tolerance = "tolerance: the tolerance must be a number not below 0, not -1; tolerance=-1 at main.sp:2 is ignored, and the default stands";
    assert!(warnings.contains(&tolerance.to_string()), "{warnings:?}");
    // the level's grid is refused, so the package's stands: a refused value does not discard
    // the values before it
    assert_eq!(setting(amp, "grid"), "grid=2 (package:fixture)");
    assert_eq!(setting(level(&c, "main.sp"), "grid"), "grid=2 (package:fixture)");
    assert_eq!(setting(amp, "seconds"), "seconds=10 (default)");
    // the refused value, with where it comes from
    let refused = amp.warnings.iter().find(|w| w.key.as_deref() == Some("grid")).unwrap().refused.clone().unwrap();
    assert_eq!(refused.value, "x");
    assert_eq!((refused.from.as_str(), refused.at.as_deref()), ("level", Some("main.sp:5")));
    // the other settings of the same SOLVER-CONFIG apply
    assert_eq!(setting(amp, "beam"), "beam=6 (level)");
    assert_eq!(setting(amp, "layout"), "layout=\"tree\" (level)");
    // and so it reads in the text
    let lines = rnl_cli::solve::check_lines(&c);
    assert!(lines.contains(&"  grid=2 (package:fixture, fixture.rnl:1)".to_string()), "{lines:?}");
    assert!(lines.contains(&format!("  warning: {grid}")), "{lines:?}");
    // a warning of the level stays with its level
    assert!(!keyed(&level(&c, "main.sp").warnings).iter().any(|w| w.starts_with("grid")));
    assert!(amp.errors.is_empty() && c.ok);
}

#[test]
fn every_bad_value_of_the_command_line_is_an_error_not_only_the_first() {
    let tool = [
        ToolSettings::Config("beam=0, layout=\"zig\", spacing_x=30".into()),
        ToolSettings::Config("limit=\"work\"".into()),
        ToolSettings::Config("unknown=1".into()),
    ];
    let c = check_settings(None, &MemoryProvider::new(), &SolveOptions::default(), &tool).unwrap();
    assert!(!c.ok);
    assert_eq!(c.levels.len(), 1);
    let l = &c.levels[0];
    assert_eq!(l.title, None);
    assert_eq!(
        keyed(&l.errors),
        [
            "beam: the beam must be at least 1",
            "layout: layout must be \"rows\", \"tree\", or \"lines\"",
            "spacing_x: spacing_x must be an integer from -1 to 20",
            "limit: limit=\"work\" requires work=COUNT",
        ]
    );
    assert_eq!(keyed(&l.warnings), ["unknown: unknown solver setting unknown; ignored"]);
    // a text that does not read is one error, of no key
    let c =
        check_settings(None, &MemoryProvider::new(), &SolveOptions::default(), &[ToolSettings::Config("beam=".into())])
            .unwrap();
    assert_eq!(c.levels[0].errors.len(), 1);
    assert_eq!(c.levels[0].errors[0].key, None);
    // a value with an error is not in effect
    assert_eq!(setting(l, "beam"), "beam=4 (default)");
    // with a file, the command line's errors are every level's
    let c = check(MAIN, &tool);
    assert!(c.levels.iter().all(|l| l.errors.len() == 4), "{c:?}");
}

#[test]
fn the_old_options_text_is_checked_item_by_item() {
    let tool = [ToolSettings::Options("beam=0;grid=0;time_limit=none;tolerance=1e-3".into())];
    let c = check_settings(None, &MemoryProvider::new(), &SolveOptions::default(), &tool).unwrap();
    let l = &c.levels[0];
    assert_eq!(keyed(&l.errors).len(), 2, "{:?}", l.errors);
    assert!(l.errors.iter().all(|e| e.message.starts_with("--solver-options: ")));
    assert_eq!(l.errors.iter().map(|e| e.key.as_deref()).collect::<Vec<_>>(), [Some("beam"), Some("grid")]);
    assert_eq!(setting(l, "time_limit"), "time_limit=false (command-line)");
    assert_eq!(setting(l, "tolerance"), "tolerance=0.001 (command-line)");
    assert!(keyed(&l.warnings).iter().any(|w| w.contains("--solver-options is deprecated")));
    // a good text has no error
    let tool = [ToolSettings::Options("beam=2;time_limit=none".into())];
    assert!(check_settings(None, &MemoryProvider::new(), &SolveOptions::default(), &tool).unwrap().ok);
}

#[test]
fn the_run_limit_is_what_limit_work_and_seconds_make_together() {
    let none = |tool: &str| {
        let tool = [ToolSettings::Config(tool.into())];
        check_settings(None, &MemoryProvider::new(), &SolveOptions::default(), &tool).unwrap()
    };
    let c = none("work=1000");
    let l = &c.levels[0];
    assert_eq!(setting(l, "limit"), "limit=\"work\" (command-line)");
    assert_eq!(setting(l, "work"), "work=1000 (command-line)");
    assert!(!l.settings.contains_key("seconds"), "{l:?}");
    let c = none("work=1000, seconds=2");
    assert_eq!(keyed(&c.levels[0].errors), ["-: give limit when both seconds and work are supplied"]);
}

#[test]
fn a_solver_config_that_the_reading_fails_is_an_error_of_its_level() {
    let main = MAIN.replace("@SOLVER-CONFIG(beam=6", "@SOLVER-CONFIG(beam=[[1]]");
    let c = check(&main, &[]);
    assert!(!c.ok);
    let amp = level(&c, "amp");
    assert_eq!(amp.errors.len(), 1, "{amp:?}");
    assert!(amp.errors[0].message.contains("SOLVER-CONFIG value"), "{amp:?}");
    assert!(level(&c, "main.sp").errors.is_empty());
}

#[test]
fn a_level_before_0_4_reads_only_the_old_options() {
    let main = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n.end\n";
    let tool = [ToolSettings::Config("beam=5".into()), ToolSettings::Options("grid=2".into())];
    let c = check(main, &tool);
    let l = &c.levels[0];
    assert_eq!(setting(l, "beam"), "beam=4 (default)");
    assert_eq!(setting(l, "grid"), "grid=2 (command-line)");
    assert!(keyed(&l.warnings).contains(&"beam: a level before 0.4 reads no --solver-config; beam is ignored".into()));
    // it is laid out in rows, not by the default of 0.4, and whatever the old options ask
    assert_eq!(setting(l, "layout"), "layout=\"rows\" (default)");
    assert!(!keyed(&l.warnings).iter().any(|w| w.starts_with("layout:")), "{:?}", l.warnings);
    let c = check(main, &[ToolSettings::Options("layout=lines".into())]);
    let l = &c.levels[0];
    assert_eq!(setting(l, "layout"), "layout=\"rows\" (default)");
    assert!(keyed(&l.warnings).contains(&"layout: a level before 0.4 is laid out in rows; layout is ignored".into()));
}

#[test]
fn a_spacing_is_warned_of_only_where_the_layout_does_not_space_by_it() {
    let main = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a 0 1k\n.end\n";
    let unused = |c: &Checked| keyed(&c.levels[0].warnings).iter().any(|w| w.contains("keep their fixed gaps"));
    assert!(!unused(&check(main, &[ToolSettings::Config("spacing_x=2".into())])));
    assert!(!unused(&check(main, &[ToolSettings::Config("spacing_x=2,layout=\"lines\"".into())])));
    assert!(unused(&check(main, &[ToolSettings::Config("spacing_x=2,layout=\"rows\"".into())])));
    assert!(unused(&check(main, &[ToolSettings::Config("spacing_x=2,layout=\"tree\"".into())])));
}

#[test]
fn a_title_that_no_level_has_is_refused() {
    let options = SolveOptions { std: false, level: Some("none".into()), ..SolveOptions::default() };
    let e = check_settings(Some(Path::new("main.sp")), &files(MAIN), &options, &[]).unwrap_err();
    assert_eq!(e, "no drawing level is titled none");
}

mod command {
    use std::process::{Command, Output};

    fn rnl(args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap()
    }

    #[test]
    fn exits_with_1_on_an_error_with_0_without_and_with_2_on_a_wrong_command_line() {
        let out = rnl(&["solve", "--check", "--json", "--solver-config", "beam=0"]);
        assert_eq!(out.status.code(), Some(1));
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["levels"][0]["title"], serde_json::Value::Null);
        assert_eq!(json["levels"][0]["errors"][0]["key"], "beam");
        assert_eq!(json["levels"][0]["settings"]["beam"], serde_json::json!({"value": 4, "from": "default"}));
        let out = rnl(&["solve", "--check", "--solver-config", "beam=2"]);
        assert_eq!(out.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&out.stdout).contains("beam=2 (command-line)"));
        for wrong in [
            &["solve", "--check", "-o", "out"][..],
            &["solve", "--check", "--in-place"],
            &["solve", "--check", "--progress"],
            &["solve", "--json", "x.sp", "-o", "out"],
            &["check", "--check", "x.sp"],
        ] {
            assert_eq!(rnl(wrong).status.code(), Some(2), "{wrong:?}");
        }
    }

    #[test]
    fn writes_nothing() {
        let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("solve_check04");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.sp");
        let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(beam=6) }\nR1 a 0 1k\n.end\n";
        std::fs::write(&file, text).unwrap();
        let out = rnl(&["solve", &file.to_string_lossy(), "--check", "--no-std"]);
        assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(String::from_utf8_lossy(&out.stdout).contains("beam=6 (header, "));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }
}
