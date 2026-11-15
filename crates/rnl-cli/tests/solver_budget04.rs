//! Time-to-work replay, effective tool settings, and complete persisted file identity.
use rnl_cli::solve::{solve_with_clock, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{Clock, SolverOptions, StoppedClock};
use std::{cell::Cell, path::Path, process::Command};

struct Ticks(Cell<u32>);
impl Clock for Ticks {
    fn elapsed(&self) -> f64 {
        let value = self.0.get();
        self.0.set(value + 1);
        f64::from(value)
    }
}

fn source() -> MemoryProvider {
    MemoryProvider::new().with("main.sp", "* budget\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(spacing_x=3,work=0) }\n* @RNL { $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) {}; %a=(0,0); %b=(5,2); {%a %b} @ALIGN(\"x\"); }\n")
}

#[test]
fn time_work_replay_preserves_the_entire_file_and_hash() {
    let files = source();
    for seconds in [0, 1, 5, 20, 100] {
        let settings = format!("seconds={seconds},spacing_y=2,junction=\"$DOT\"");
        let options = SolveOptions {
            std: false,
            solver: SolverOptions::default().with_config(&settings).unwrap(),
            ..SolveOptions::default()
        };
        let timed = solve_with_clock(Path::new("main.sp"), &files, &options, &Ticks(Cell::new(0))).unwrap();
        assert_eq!(timed.errors, 0, "{timed:?}");
        let level = &timed.levels[0];
        assert_eq!(level.problem.junction.effective().definition.as_deref(), Some("$DOT"));
        assert!(level.solved.report.warnings.iter().any(|s| s.contains("tool-only")));
        let replay = level.solved.report.replay_config().unwrap();
        let settings = format!("{replay},spacing_y=2,junction=\"$DOT\"");
        let replay = SolveOptions { solver: SolverOptions::default().with_config(&settings).unwrap(), ..options };
        let again = solve_with_clock(Path::new("main.sp"), &files, &replay, &StoppedClock).unwrap();
        assert_eq!(level.hash, again.levels[0].hash, "cutoff {seconds}");
        assert_eq!(timed.files[0].text, again.files[0].text, "cutoff {seconds}");
        let saved = MemoryProvider::new().with("main.sp", &timed.files[0].text);
        let view_options = rnl_view::BuildOptions {
            status: true,
            svg: false,
            runtime: rnl_runtime::Options { std: false, ..Default::default() },
            ..Default::default()
        };
        let view = rnl_view::build("main.sp", &saved, &view_options).unwrap();
        assert_eq!(view.levels[0].solve, Some("solved"), "tool overrides must not make a saved result stale");
        let changed = MemoryProvider::new().with("main.sp", &timed.files[0].text.replace("spacing_x=3", "spacing_x=4"));
        assert_eq!(rnl_view::build("main.sp", &changed, &view_options).unwrap().levels[0].solve, Some("stale"));
    }
}

#[test]
fn defaults_command_prints_accepted_named_parameters() {
    let rnl =
        |args: &[&str]| Command::new(env!("CARGO_BIN_EXE_rnl")).arg("solver-defaults").args(args).output().unwrap();
    let output = rnl(&[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8(output.stdout).unwrap();
    // a line for each layout, the default first, of the settings that layout uses
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    for (line, layout) in lines.iter().zip(["lines", "rows", "tree"]) {
        assert!(line.starts_with(&format!("layout=\"{layout}\", ")), "{line}");
        assert!(SolverOptions::default().with_config(line).is_ok(), "{line}");
        assert!(line.contains("seconds=10"), "{line}");
        // safety_seconds is of the limits work and seed, not of the default time
        assert!(!line.contains("safety_seconds"), "{line}");
        assert_eq!(line.contains("beam=4"), layout == "rows", "{line}");
        assert_eq!(line.contains("candidates=256"), layout == "rows", "{line}");
        assert_eq!(line.contains("spacing_x=0"), layout == "lines", "{line}");
    }
    // --layout gives the line of one layout
    assert_eq!(String::from_utf8(rnl(&["--layout", "rows"]).stdout).unwrap().trim(), lines[1]);
    // and, with --json, the rows of the table that apply to it
    let table: serde_json::Value = serde_json::from_slice(&rnl(&["--json", "--layout", "tree"]).stdout).unwrap();
    assert_eq!(table["layout"], "tree");
    let names = |list: &str, key: &str| -> Vec<String> {
        table[list].as_array().unwrap().iter().map(|r| r[key].as_str().unwrap().to_string()).collect()
    };
    assert!(names("settings", "key").contains(&"grid".to_string()));
    assert!(!names("settings", "key").contains(&"beam".to_string()));
    assert!(names("objectives", "name").contains(&"turns".to_string()));
    assert!(names("stages", "name").contains(&"tree".to_string()));
    assert!(!names("stages", "name").contains(&"lines".to_string()));
    assert!(table.get("lines_order").is_none());
    let lines_table: serde_json::Value = serde_json::from_slice(&rnl(&["--json", "--layout", "lines"]).stdout).unwrap();
    assert!(!lines_table["objectives"].as_array().unwrap().iter().any(|o| o["name"] == "turns"));
    assert!(lines_table["lines_order"]["ranks"].is_array());
    // without --layout, the whole table, as before
    let whole: serde_json::Value = serde_json::from_slice(&rnl(&["--json"]).stdout).unwrap();
    assert!(whole.get("layout").is_none());
    assert_eq!(whole, rnl_cli::solver_table::settings_table_json());
    // a layout the solver does not have is refused; --layout is an option of solver-defaults only
    assert_eq!(rnl(&["--layout", "grid"]).status.code(), Some(2));
    let check = Command::new(env!("CARGO_BIN_EXE_rnl")).args(["check", "x.sp", "--layout", "rows"]).output().unwrap();
    assert_eq!(check.status.code(), Some(2));
}

#[test]
fn legacy_explicit_options_do_not_add_settings_to_the_input_hash() {
    for version in ["0.2", "0.3"] {
        let text = format!(
            "* old settings\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ %a=(0,0); %b=(5,2); }}\n"
        );
        let files = MemoryProvider::new().with("main.sp", &text);
        let values = "beam=4;grid=1;objectives=none;time_limit=none";
        let plain = SolveOptions {
            std: false,
            solver: SolverOptions::default().with_overrides(values).unwrap(),
            ..Default::default()
        };
        let explicit =
            SolveOptions { solver: SolverOptions::default().with_legacy_config(values).unwrap(), ..plain.clone() };
        let before = solve_with_clock(Path::new("main.sp"), &files, &plain, &StoppedClock).unwrap();
        let after = solve_with_clock(Path::new("main.sp"), &files, &explicit, &StoppedClock).unwrap();
        assert!(after.levels[0].problem.solver_config.is_empty());
        assert_eq!(before.levels[0].hash, after.levels[0].hash, "{version}");
        assert_eq!(before.files[0].text, after.files[0].text, "{version}");
    }
}

/// A level before 0.4 is laid out in rows whatever layout the old options name, so the layout is
/// not in the options its hash covers: the record is the one written with no layout named.
#[test]
fn a_layout_named_for_a_level_before_0_4_is_not_in_its_hash() {
    for version in ["0.2", "0.3"] {
        let text = format!(
            "* old settings\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ %a=(0,0); %b=(5,2); }}\n"
        );
        let files = MemoryProvider::new().with("main.sp", &text);
        let values = "beam=4;grid=1;objectives=none;time_limit=none";
        let solve = |values: &str| {
            let options = SolveOptions {
                std: false,
                solver: SolverOptions::default().with_overrides(values).unwrap(),
                ..Default::default()
            };
            solve_with_clock(Path::new("main.sp"), &files, &options, &StoppedClock).unwrap()
        };
        let plain = solve(values);
        for layout in ["rows", "tree", "lines"] {
            let named = solve(&format!("{values};layout={layout}"));
            assert_eq!(plain.levels[0].hash, named.levels[0].hash, "{version} {layout}");
            assert_eq!(plain.files[0].text, named.files[0].text, "{version} {layout}");
        }
    }
}

#[test]
fn junction_tool_metadata_changes_options_and_writeback_but_not_file_identity() {
    use rnl_runtime::{
        problem::{input_digest, sig2},
        solution::{write_record, Branch, Solution},
    };
    let files=MemoryProvider::new().with("main.sp", "* junction tools\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$A\") }\n* @RNL { $A = @DEFINE(roles=[[\"p\",\"point\"]]) {}; $B = @DEFINE(roles=[[\"p\",\"point\"]]) {}; $ALIAS = $B; %a=(0,0); }\n");
    let mut results = Vec::new();
    for (request, name) in [("$A", "$A"), ("$B", "$B"), ("", ""), ("$ALIAS", "$ALIAS")] {
        let config = format!("junction=\"{request}\",spacing_x=4");
        let runtime = rnl_runtime::Options {
            std: false,
            solver_config: rnl_runtime::solver_settings::parse_overrides(&config).unwrap(),
            ..rnl_runtime::Options::solver()
        };
        let read = rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &runtime).unwrap();
        let p = &read.problems[0];
        assert_eq!(p.junction.definition.as_deref(), Some("$A"));
        assert_eq!(p.junction.effective().definition.as_deref(), (!name.is_empty()).then_some(name));
        let from_metadata = SolverOptions::default().for_problem(p).0;
        assert_eq!(from_metadata.settings.junction.as_deref(), Some(name));
        let options = SolverOptions::default().with_config(&config).unwrap().for_problem(p).0;
        let canonical = options.canonical();
        assert_eq!(SolverOptions::parse(&canonical).unwrap().settings.junction.as_deref(), Some(name));
        let hash = sig2(p, &canonical);
        let solution = Solution { branches: vec![Branch { at: (2., 3.), junction: true }], ..Default::default() };
        let record = write_record(p, &solution, rnl_solver::AUTHOR, &hash).record;
        if name.is_empty() {
            assert!(!record.contains("@ATTACH"));
        } else {
            assert!(record.contains(&format!("{name} @ATTACH")), "{record}");
        }
        results.push((input_digest(p), hash));
    }
    assert!(results.iter().all(|r| r.0 == results[0].0));
    assert_ne!(results[0].1, results[1].1);
    assert_ne!(results[1].1, results[2].1);
    // The writable alias itself is preserved in the record, so it is a distinct option identity.
    assert_ne!(results[1].1, results[3].1);
}
