//! The commands and options of `rnl` that solve, or show what a solver reads: `problem`, `solve`
//! and `preview`, their part of the usage, and the options only they take. `--level`, `-o` and
//! `--encoding` are shared with other commands and stay with them.

use std::path::PathBuf;
use std::process::ExitCode;

use rnl_cli::solve::{SolveOptions, Target, WallClock};
use rnl_cli::FsProvider;
use rnl_runtime::diag::codes;
use rnl_runtime::Options;
use rnl_solver::SolverOptions;

use crate::{needs, Args, Said};

struct ProgressClock<'a> {
    inner: &'a dyn rnl_solver::Clock,
    enabled: bool,
}

impl rnl_solver::Clock for ProgressClock<'_> {
    fn check_every(&self) -> u64 {
        self.inner.check_every()
    }
    fn advances(&self) -> bool {
        self.inner.advances()
    }
    fn elapsed(&self) -> f64 {
        self.inner.elapsed()
    }
    fn profiler(&self) -> Option<rnl_solver::profile::ProfileHandle> {
        self.inner.profiler()
    }
    fn progress(&self, p: rnl_solver::BudgetProgress) {
        // the text lines tell of the work only, as they always have
        if self.enabled && matches!(p.kind, rnl_solver::ProgressKind::Work | rnl_solver::ProgressKind::Complete) {
            eprintln!("solver: {}: {} work units", p.stage, p.work);
        }
    }
}

/// The usage of the solver's commands, after the other commands' options and before the saving
/// options.
macro_rules! usage {
    () => {
        "problem options (the levels are read without their type=\"generated\" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  key=value;... over the solver's defaults, as for solve: the options the hash covers
  --previous          give each level what its type=\"generated\" record decided, as hints
  --solver-input      emit the lossless input protocol for the independent rnl-solver program
  --solver-config S   RNL named parameters; repeated options merge by key, the last value wins
solve options (the levels are read without their type=\"generated\" records; -o or --in-place is needed):
  -o DIR              write the files solved under DIR, named as they are relative to FILE's directory,
                      with a copy of the other files they read from there, such as includes
  --in-place          write them over the files read instead
  --solver-config S   0.4 settings, for example limit=\"time\",seconds=10 or limit=\"work\",work=100000
                      overrides file/package settings by key; run limits are tool-only
  --progress          report 0.4 stages and consumed work on standard error
  --progress=json     report the solve on standard error as events, one JSON object per line
                      (docs/solver-progress.md; solve and preview take it)
  --external-solver P run executable P with the solver input/result JSON protocol
  --external-arg ARG  append one literal argument to P (repeatable; no shell)
  --external-timeout S  stop P after S seconds per level (120 by default)
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
    pub external: Option<PathBuf>,
    pub external_args: Vec<String>,
    pub external_timeout: Option<f64>,
    pub solver_input: bool,
    pub progress: bool,
    pub progress_json: bool,
    pub configs: Vec<(bool, String)>,
    /// As given; `problem` and `solve` each have their own default.
    pub in_place: bool,
    pub report: bool,
    pub profile: Option<PathBuf>,
    pub previous: bool,
}

impl SolverArgs {
    /// Take `option`, with its value from `rest` when it has one: true when it is one of these.
    pub fn take(&mut self, option: &str, rest: &mut impl Iterator<Item = String>) -> Result<bool, Said> {
        match option {
            "--external-solver" => {
                self.external =
                    Some(PathBuf::from(rest.next().ok_or_else(|| needs("--external-solver needs a path"))?));
            }
            "--external-arg" => {
                self.external_args.push(rest.next().ok_or_else(|| needs("--external-arg needs an argument"))?);
            }
            "--external-timeout" => {
                let value = rest.next().ok_or_else(|| needs("--external-timeout needs seconds"))?;
                let seconds: f64 =
                    value.parse().map_err(|_| needs("--external-timeout needs positive finite seconds"))?;
                if !seconds.is_finite() || seconds <= 0. || std::time::Duration::try_from_secs_f64(seconds).is_err() {
                    return Err(needs("--external-timeout needs positive finite seconds"));
                }
                self.external_timeout = Some(seconds);
            }
            "--progress" => self.progress = true,
            "--progress=json" => self.progress_json = true,
            "--solver-config" => {
                self.configs.push((false, rest.next().ok_or_else(|| needs("--solver-config needs a text"))?))
            }
            "--previous" => self.previous = true,
            "--solver-input" => self.solver_input = true,
            "--solver-options" => {
                self.configs.push((true, rest.next().ok_or_else(|| needs("--solver-options needs a text"))?))
            }
            "--in-place" => self.in_place = true,
            "--report" => self.report = true,
            "--profile" => {
                self.profile = Some(PathBuf::from(rest.next().ok_or_else(|| needs("--profile needs a JSON path"))?))
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Refuse what `command` does not take.
    pub fn check(&self, command: &str) -> Result<(), Said> {
        if self.external.is_none() && (!self.external_args.is_empty() || self.external_timeout.is_some()) {
            return Err(needs("--external-arg and --external-timeout require --external-solver"));
        }
        if self.external.is_some() && self.profile.is_some() {
            return Err(needs("--profile observes the embedded solver; it cannot be combined with --external-solver"));
        }
        if self.solver_input && self.previous {
            return Err("--previous is available only in the readable problem format; omit --solver-input".into());
        }
        if self.profile.is_some() && command != "solve" {
            return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, "--profile is an option of solve"));
        }
        Ok(())
    }
}

/// `rnl problem`: what a solver reads of each level, with its hash, as JSON.
/// Options the solver does not take.
fn solver_options(e: String) -> Said {
    Said::new(codes::CLI_SOLVER_OPTIONS_INVALID, format!("--solver-options: {e}"))
}

fn configured(args: &Args) -> Result<SolverOptions, Said> {
    let mut options = SolverOptions::default();
    for (legacy, text) in &args.solver.configs {
        let named_syntax = !text.contains(';')
            && rnl_runtime::solver_settings::parse_overrides(text).is_ok_and(|settings| {
                settings.len() > 1
                    || settings.iter().any(|s| {
                        matches!(
                            s.key.as_str(),
                            "limit"
                                | "work"
                                | "seconds"
                                | "safety_seconds"
                                | "candidates"
                                | "astar_expansions"
                                | "junction"
                                | "layout"
                        ) || matches!(
                            s.value,
                            rnl_runtime::interp::obj::Lit::Str(_)
                                | rnl_runtime::interp::obj::Lit::Bool(_)
                                | rnl_runtime::interp::obj::Lit::Arr(_)
                        )
                    })
            });
        options = if *legacy && !named_syntax { options.with_legacy_config(text) } else { options.with_config(text) }
            .map_err(solver_options)?;
        options.settings.legacy_alias |= *legacy;
    }
    Ok(options)
}

pub(crate) fn problem(args: &Args, provider: &FsProvider) -> Result<ExitCode, Said> {
    // the options as the solver's canonical text writes them, so that the hash is the one
    // `rnl solve` writes with the same options
    let solver = configured(args)?;
    let options = Options {
        solver_config: solver
            .settings
            .overrides
            .iter()
            .filter(|s| !rnl_solver::is_run_setting(&s.key))
            .cloned()
            .collect(),
        std: args.std,
        include_std: args.include_std,
        std_version: args.std_version.clone(),
        ..Options::solver()
    };
    let (file, dialect) = (args.file.to_string_lossy(), args.dialect.as_deref());
    let (r, previous) = if args.solver.previous {
        rnl_view::read_previous_with(&file, dialect, &args.formats, provider, &options)?
    } else {
        (rnl_cli::interpret_file_with(&args.file, dialect, &args.formats, provider, &options)?, Vec::new())
    };
    if args.solver.solver_input {
        let diagnostics = r.diagnostics();
        for diagnostic in &diagnostics {
            eprintln!("{}", r.sink.render(diagnostic));
        }
        if diagnostics.iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error) {
            return Ok(ExitCode::from(1));
        }
        let levels: Vec<_> = r
            .problems
            .iter()
            .filter(|p| args.level.as_ref().is_none_or(|t| *t == p.title))
            .map(|p| rnl_solver_io::LevelInput { problem: p.clone(), options: solver.clone() })
            .collect();
        if levels.is_empty() {
            if let Some(title) = &args.level {
                return Err(Said::new(codes::SOLVE_NO_LEVEL_TITLED, format!("no drawing level is titled {title}")));
            }
        }
        let input = rnl_solver_io::encode_input(&rnl_solver_io::InputDocument { levels })
            .map_err(|e| Said::from(format!("cannot export solver input: {e}")))?;
        println!("{}", serde_json::to_string_pretty(&input).unwrap());
        return Ok(ExitCode::SUCCESS);
    }
    let levels: Vec<_> = r
        .problems
        .iter()
        .enumerate()
        .filter(|(_, p)| args.level.as_ref().is_none_or(|t| *t == p.title))
        .map(|(k, p)| {
            let mut level = rnl_cli::problem::level(p, &solver.for_problem(p).0.canonical());
            if args.solver.previous {
                let hints = rnl_cli::problem::previous(previous.get(k).and_then(Option::as_ref));
                level.as_object_mut().unwrap().insert("previous".into(), hints);
            }
            level
        })
        .collect();
    if levels.is_empty() {
        if let Some(t) = &args.level {
            return Err(Said::new(codes::SOLVE_NO_LEVEL_TITLED, format!("no drawing level is titled {t}")));
        }
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "levels": levels })).unwrap());
    Ok(ExitCode::SUCCESS)
}

/// `rnl preview`: as `rnl solve` solves, and nothing written: the Document of the files as solved
/// and the report, which the desktop viewer shows (decision 0010).
pub(crate) fn preview(args: &Args, provider: &FsProvider) -> Result<ExitCode, Said> {
    let solver = configured(args)?;
    let options = SolveOptions {
        dialect: args.dialect.clone(),
        formats: args.formats.clone(),
        std: args.std,
        include_std: args.include_std,
        level: args.level.clone(),
        solver,
    };
    let entry = rnl_cli::path_str(&args.file);
    eprintln!("{}", rnl_cli::SOLVER_PREVIEW_NOTICE);
    if !args.solver.progress_json {
        let shown = rnl_cli::preview::preview(&entry, provider, &options, &WallClock::new(), &|_| true)?;
        let out = serde_json::json!({ "document": shown.document, "report": shown.report });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return Ok(ExitCode::SUCCESS);
    }
    // with --progress=json, as for solve, the events go on the standard error and the run ends with `done`,
    // after `error` when it fails
    let started = std::time::Instant::now();
    let emit = |e: rnl_cli::progress::Event| eprintln!("{}", e.line());
    let done = |exit| rnl_cli::progress::Event::Done {
        seconds: started.elapsed().as_secs_f64(),
        exit: Some(exit),
        written: None,
    };
    let events = |e: &rnl_cli::progress::Event| {
        eprintln!("{}", e.line());
        true
    };
    match rnl_cli::preview::preview_progress(&entry, provider, &options, &WallClock::new(), &events) {
        Ok(shown) => {
            let out = serde_json::json!({ "document": shown.document, "report": shown.report });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            emit(done(0));
            Ok(ExitCode::SUCCESS)
        }
        Err(message) => {
            emit(rnl_cli::progress::Event::Error { message: message.clone() });
            emit(done(2));
            Err(message.into())
        }
    }
}

/// `rnl solve`: solve, print what became of each level, write the files (plan 5.10). Exits with 1
/// when any error was reported, once the files are written: an error of the netlist as a solver
/// reads it, counted as `rnl check --no-generated` counts them (7.6), or an error of the solver, a
/// fact that contradicts itself (the draft 0.3, 8.5; 7.6); with 2 when a file cannot be read or
/// written.
pub(crate) fn solve(args: &Args, provider: &FsProvider) -> Result<ExitCode, Said> {
    if !args.solver.progress_json {
        return solve_with(args, provider).map(|(code, _, _)| code);
    }
    // with --progress=json, every run ends with `done`, after `error` when it fails
    let started = std::time::Instant::now();
    let emit = |e: rnl_cli::progress::Event| eprintln!("{}", e.line());
    let done = |exit, written| rnl_cli::progress::Event::Done {
        seconds: started.elapsed().as_secs_f64(),
        exit: Some(exit),
        written,
    };
    match solve_with(args, provider) {
        Ok((code, exit, written)) => {
            emit(done(exit, Some(written)));
            Ok(code)
        }
        Err(said) => {
            emit(rnl_cli::progress::Event::Error { message: said.to_string() });
            emit(done(2, None));
            Err(said)
        }
    }
}

/// [`solve`]: the exit code, as a number too, and how many files were written.
fn solve_with(args: &Args, provider: &FsProvider) -> Result<(ExitCode, u8, usize), Said> {
    let target = match (&args.out, args.solver.in_place) {
        (Some(_), true) => {
            let text = "solve writes under -o DIR or --in-place, not both";
            return Err(Said::new(codes::CLI_SOLVE_TARGET_BOTH, text));
        }
        (Some(dir), false) => Target::Dir(dir.clone()),
        (None, true) => Target::InPlace,
        (None, false) => {
            let text = "solve needs -o DIR, to write the files solved under DIR, or --in-place, to write them over the files read";
            return Err(Said::new(codes::CLI_SOLVE_TARGET_MISSING, text));
        }
    };
    let solver = configured(args)?;
    // the groups of the facts of priority low are sought only for their hints (8.5, 7.6)
    let solver = SolverOptions { hints_for_low: args.solver.report, ..solver };
    let options = SolveOptions {
        dialect: args.dialect.clone(),
        formats: args.formats.clone(),
        std: args.std,
        include_std: args.include_std,
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
                return Err(Said::new(codes::CLI_PROFILE_SAME_AS_INPUT, "--profile must differ from the input file"));
            }
            rnl_cli::profile::ProfileClock::new(path).map_err(Said::from)
        })
        .transpose()?;
    eprintln!("{}", rnl_cli::SOLVER_PREVIEW_NOTICE);
    let wall_clock = WallClock::new();
    let clock: &dyn rnl_solver::Clock = profile_clock.as_ref().map_or(&wall_clock as &dyn rnl_solver::Clock, |c| c);
    let observed = ProgressClock { inner: clock, enabled: args.solver.progress && !args.solver.progress_json };
    let events = |e: &rnl_cli::progress::Event| {
        eprintln!("{}", e.line());
        true
    };
    let result = if args.solver.progress_json {
        let external = args.solver.external.as_ref();
        rnl_cli::progress::solve_progress_using(
            &args.file,
            provider,
            &options,
            &observed,
            &events,
            &|problem, options, clock| match external {
                Some(program) => rnl_cli::external_solver::solve(
                    program,
                    &args.solver.external_args,
                    args.solver.external_timeout.unwrap_or(120.),
                    problem,
                    options,
                ),
                None => Ok(rnl_cli::solve::BackendResult {
                    solved: rnl_solver::solve_with_clock(problem, options, clock),
                    author: rnl_solver::AUTHOR.to_string(),
                    report_available: true,
                }),
            },
        )
    } else if let Some(program) = &args.solver.external {
        rnl_cli::solve::solve_using(&args.file, provider, &options, &|_| true, &|problem, options| {
            rnl_cli::external_solver::solve(
                program,
                &args.solver.external_args,
                args.solver.external_timeout.unwrap_or(120.),
                problem,
                options,
            )
        })
    } else {
        rnl_cli::solve::solve_with_clock(&args.file, provider, &options, &observed)
    };
    if let Some(clock) = &profile_clock {
        clock.finish(result.is_ok())?;
    }
    let run = result?;
    // nothing is written when a file has no place to go
    let output_paths = rnl_cli::solve::all_destinations(&run, &target)?;
    if let Some(profile) = &args.solver.profile {
        let profile = std::fs::canonicalize(profile)
            .map_err(|e| Said::new(codes::CLI_PROFILE_PATH_UNRESOLVED, format!("cannot resolve profile path: {e}")))?;
        if output_paths.iter().any(|p| std::fs::canonicalize(p).is_ok_and(|p| p == profile)) {
            return Err(Said::new(
                codes::CLI_PROFILE_CONFLICTS_OUTPUT,
                "--profile conflicts with a solved output path",
            ));
        }
    }
    if args.solver.external.is_none() && run.levels.iter().all(|l| l.solved.report.budget.is_none()) {
        println!("solver options: {}", options.solver.canonical());
    }
    if run.levels.is_empty() && run.not_solved.is_empty() {
        println!("hint: {}: {} has no drawing level to solve", codes::CLI_SOLVE_NO_LEVEL, run.entry);
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
                "warning: {}: {} reads {path}, which is outside its directory and is not copied under {}",
                codes::CLI_SOLVE_OUTSIDE_NOT_COPIED,
                run.entry,
                rnl_cli::path_str(dir)
            );
        }
    }
    if run.errors > 0 {
        eprintln!(
            "error: {}: {} errors in {} as a solver reads it; run `rnl check --no-generated` to see them",
            codes::CLI_SOLVE_NETLIST_ERRORS,
            run.errors,
            run.entry
        );
    }
    // an error of the netlist, or a fact that contradicts itself (8.5, 7.6): the levels are written
    // all the same
    let written = writing.written.len();
    match writing.error {
        Some(e) => Err(e.into()),
        None if run.errors + rnl_cli::solve::errors(&run) > 0 => Ok((ExitCode::from(1), 1, written)),
        None => Ok((ExitCode::SUCCESS, 0, written)),
    }
}
