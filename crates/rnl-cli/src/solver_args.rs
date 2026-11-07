//! The commands and options of `rnl` that solve, or show what a solver reads: `problem`, `solve`
//! and `preview`, their part of the usage, and the options only they take. `--level`, `-o` and
//! `--encoding` are shared with other commands and stay with them.

use std::path::PathBuf;
use std::process::ExitCode;

use rnl_cli::solve::{SolveOptions, Target, WallClock};
use rnl_cli::FsProvider;
use rnl_runtime::Options;
use rnl_solver::SolverOptions;

use crate::Args;

/// The usage of the solver's commands, after the other commands' options and before the saving
/// options.
macro_rules! usage {
    () => {
        "problem options (the levels are read without their type=\"generated\" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  key=value;... over the solver's defaults, as for solve: the options the hash covers
  --previous          give each level what its type=\"generated\" record decided, as hints
solve options (the levels are read without their type=\"generated\" records; -o or --in-place is needed):
  -o DIR              write the files solved under DIR, named as they are relative to FILE's directory,
                      with a copy of the other files they read from there, such as includes
  --in-place          write them over the files read instead
  --level TITLE       only the level with this title
  --solver-options S  key=value;... over the solver's defaults, for tolerance, grid, objectives, beam,
                      budget and time_limit, as rnl-solver's canonical text writes them; the hash
                      covers them. time_limit is the seconds the resolution of a level may spend on
                      checking (8.5), 5 by default, or none for no limit: a fact whose check does not
                      finish within it is discarded with a warning, and the result may then differ
                      from one run to the next; when the conflict groups of a fact are not all found
                      within it, its hint says that the list is not complete
  --profile OUT.json  checkpoint solver timings and counters (solve only; may affect a wall-clock deadline)
  --report            print what became of every fact, not only of the facts discarded, and the hints
                      of the facts of priority low and of the statements only they lose (8.5), whose
                      minimal conflict groups the solver then seeks all, not only the first
  a fact that has no solution on its own contradicts itself and fails (8.5): solve prints an error line
  for it, and solves the level and writes it back without it
  solve exits with 1 when an error was reported, once the files are written: one of the netlist as a
  solver reads it, whose number it gives on the standard error as rnl check --no-generated counts
  them, or one of the solver
  the solver is a preview, whose layouts may be poor: solve and preview say so in a line on the
  standard error each time they solve; what they print on the standard output and write is the same
"
    };
}
pub(crate) use usage;

/// The options only the solver's commands take.
#[derive(Default)]
pub(crate) struct SolverArgs {
    /// As given; `problem` and `solve` each have their own default.
    pub solver_options: Option<String>,
    pub in_place: bool,
    pub report: bool,
    pub profile: Option<PathBuf>,
    pub previous: bool,
}

impl SolverArgs {
    /// Take `option`, with its value from `rest` when it has one: true when it is one of these.
    pub fn take(&mut self, option: &str, rest: &mut impl Iterator<Item = String>) -> Result<bool, String> {
        match option {
            "--previous" => self.previous = true,
            "--solver-options" => self.solver_options = Some(rest.next().ok_or("--solver-options needs a text")?),
            "--in-place" => self.in_place = true,
            "--report" => self.report = true,
            "--profile" => self.profile = Some(PathBuf::from(rest.next().ok_or("--profile needs a JSON path")?)),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Refuse what `command` does not take.
    pub fn check(&self, command: &str) -> Result<(), String> {
        if self.profile.is_some() && command != "solve" {
            return Err("--profile is an option of solve".into());
        }
        Ok(())
    }
}

/// `rnl problem`: what a solver reads of each level, with its hash, as JSON.
pub(crate) fn problem(args: &Args, provider: &FsProvider) -> Result<ExitCode, String> {
    // the options as the solver's canonical text writes them, so that the hash is the one
    // `rnl solve` writes with the same options
    let problem_options = SolverOptions::default()
        .with_overrides(args.solver.solver_options.as_deref().unwrap_or(""))
        .map_err(|e| format!("--solver-options: {e}"))?
        .canonical();
    let options = Options { std: args.std, std_version: args.std_version.clone(), ..Options::solver() };
    let (file, dialect) = (args.file.to_string_lossy(), args.dialect.as_deref());
    let (r, previous) = if args.solver.previous {
        rnl_view::read_previous_with(&file, dialect, &args.formats, provider, &options)?
    } else {
        (rnl_cli::interpret_file_with(&args.file, dialect, &args.formats, provider, &options)?, Vec::new())
    };
    let levels: Vec<_> = r
        .problems
        .iter()
        .enumerate()
        .filter(|(_, p)| args.level.as_ref().is_none_or(|t| *t == p.title))
        .map(|(k, p)| {
            let mut level = rnl_cli::problem::level(p, &problem_options);
            if args.solver.previous {
                let hints = rnl_cli::problem::previous(previous.get(k).and_then(Option::as_ref));
                level.as_object_mut().unwrap().insert("previous".into(), hints);
            }
            level
        })
        .collect();
    if levels.is_empty() {
        if let Some(t) = &args.level {
            return Err(format!("no drawing level is titled {t}"));
        }
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "levels": levels })).unwrap());
    Ok(ExitCode::SUCCESS)
}

/// `rnl preview`: as `rnl solve` solves, and nothing written: the Document of the files as solved
/// and the report, which the desktop viewer shows (decision 0010).
pub(crate) fn preview(args: &Args, provider: &FsProvider) -> Result<ExitCode, String> {
    let solver = SolverOptions::default()
        .with_overrides(args.solver.solver_options.as_deref().unwrap_or(""))
        .map_err(|e| format!("--solver-options: {e}"))?;
    let options = SolveOptions {
        dialect: args.dialect.clone(),
        formats: args.formats.clone(),
        std: args.std,
        level: args.level.clone(),
        solver,
    };
    let entry = rnl_cli::path_str(&args.file);
    eprintln!("{}", rnl_cli::SOLVER_PREVIEW_NOTICE);
    let shown = rnl_cli::preview::preview(&entry, provider, &options, &WallClock::new(), &|_| true)?;
    let out = serde_json::json!({ "document": shown.document, "report": shown.report });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(ExitCode::SUCCESS)
}

/// `rnl solve`: solve, print what became of each level, write the files (plan 5.10). Exits with 1
/// when any error was reported, once the files are written: an error of the netlist as a solver
/// reads it, counted as `rnl check --no-generated` counts them (7.6), or an error of the solver, a
/// fact that contradicts itself (the draft 0.3, 8.5; 7.6); with 2 when a file cannot be read or
/// written.
pub(crate) fn solve(args: &Args, provider: &FsProvider) -> Result<ExitCode, String> {
    let target = match (&args.out, args.solver.in_place) {
        (Some(_), true) => return Err("solve writes under -o DIR or --in-place, not both".into()),
        (Some(dir), false) => Target::Dir(dir.clone()),
        (None, true) => Target::InPlace,
        (None, false) => return Err(
            "solve needs -o DIR, to write the files solved under DIR, or --in-place, to write them over the files read"
                .into(),
        ),
    };
    let solver = SolverOptions::default()
        .with_overrides(args.solver.solver_options.as_deref().unwrap_or(""))
        .map_err(|e| format!("--solver-options: {e}"))?;
    // the groups of the facts of priority low are sought only for their hints (8.5, 7.6)
    let solver = SolverOptions { hints_for_low: args.solver.report, ..solver };
    let options = SolveOptions {
        dialect: args.dialect.clone(),
        formats: args.formats.clone(),
        std: args.std,
        level: args.level.clone(),
        solver,
    };
    let profile_clock = args
        .solver
        .profile
        .as_ref()
        .map(|path| {
            if std::fs::canonicalize(path).ok().zip(std::fs::canonicalize(&args.file).ok()).is_some_and(|(a, b)| a == b)
            {
                return Err("--profile must differ from the input file".to_string());
            }
            rnl_cli::profile::ProfileClock::new(path)
        })
        .transpose()?;
    eprintln!("{}", rnl_cli::SOLVER_PREVIEW_NOTICE);
    let result = match &profile_clock {
        Some(clock) => rnl_cli::solve::solve_with_clock(&args.file, provider, &options, clock),
        None => rnl_cli::solve::solve(&args.file, provider, &options),
    };
    if let Some(clock) = &profile_clock {
        clock.finish(result.is_ok())?;
    }
    let run = result?;
    // nothing is written when a file has no place to go
    let output_paths = rnl_cli::solve::all_destinations(&run, &target)?;
    if let Some(profile) = &args.solver.profile {
        let profile = std::fs::canonicalize(profile).map_err(|e| format!("cannot resolve profile path: {e}"))?;
        if output_paths.iter().any(|p| std::fs::canonicalize(p).is_ok_and(|p| p == profile)) {
            return Err("--profile conflicts with a solved output path".into());
        }
    }
    println!("solver options: {}", options.solver.canonical());
    if run.levels.is_empty() && run.not_solved.is_empty() {
        println!("{} has no drawing level to solve", run.entry);
    }
    for line in rnl_cli::solve::lines(&run, args.solver.report) {
        println!("{line}");
    }
    let writing = rnl_cli::solve::write_with_encoding(&run, &target, args.encoding);
    for path in &writing.written {
        println!("wrote {}", rnl_cli::path_str(path));
    }
    for path in &writing.copied {
        println!("copied {}", rnl_cli::path_str(path));
    }
    if let Target::Dir(dir) = &target {
        for path in &writing.outside {
            eprintln!(
                "warning: {} reads {path}, which is outside its directory and is not copied under {}",
                run.entry,
                rnl_cli::path_str(dir)
            );
        }
    }
    if run.errors > 0 {
        eprintln!(
            "{} errors in {} as a solver reads it; run `rnl check --no-generated` to see them",
            run.errors, run.entry
        );
    }
    // an error of the netlist, or a fact that contradicts itself (8.5, 7.6): the levels are written
    // all the same
    match writing.error {
        Some(e) => Err(e),
        None if run.errors + rnl_cli::solve::errors(&run) > 0 => Ok(ExitCode::from(1)),
        None => Ok(ExitCode::SUCCESS),
    }
}
