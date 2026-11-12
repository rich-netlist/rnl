use rnl_solver::{Clock, RunLimit, StopReason};
use rnl_solver_io::{InputDocument, LevelResult, ResultDocument, SolverIdentity};
use std::{
    ffi::OsString,
    io::{Read, Write},
    process::ExitCode,
    time::Instant,
};

const HELP: &str = "rnl-solver [solve] [INPUT|-]\n\nRead a versioned RNL solver input from a file or stdin.\nWrite one solver result JSON document to stdout; diagnostics go to stderr.\nUse rnl problem --solver-input to export the input.\n\n  --help       show this help\n  --version    show this executable's independent version\n";

struct Elapsed(Instant);
impl Clock for Elapsed {
    fn elapsed(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

fn solve(input: &InputDocument) -> ResultDocument {
    let clock = Elapsed(Instant::now());
    let mut levels = Vec::new();
    for (request, level) in input.levels.iter().enumerate() {
        let mut solved = rnl_solver::solve_with_clock(&level.problem, &level.options, &clock);
        let options = solved
            .report
            .canonical_options
            .clone()
            .unwrap_or_else(|| level.options.for_problem(&level.problem).0.canonical());
        if let Some(mut budget) = solved.report.budget {
            eprintln!(
                "level {}: {} work units; limit {:?}; stopped {:?}",
                level.problem.level, budget.work, budget.limit, budget.reason
            );
            // Persist replay settings, not the wall-clock observation. Time and its work
            // replay therefore produce identical result bytes. The observation is on stderr.
            if matches!(budget.limit, RunLimit::Time(_) | RunLimit::Work(_)) && budget.reason != StopReason::Safety {
                budget.limit = RunLimit::Work(budget.work);
                if budget.reason == StopReason::Time {
                    budget.reason = StopReason::Work;
                }
                solved.report.budget = Some(budget);
                solved.report.notes.retain(|n| !n.starts_with("budget:"));
                solved.report.notes.push(format!(
                    "budget: {} work units; replay with limit=\"work\", work={}",
                    budget.work, budget.work
                ));
            }
        }
        levels.push(LevelResult {
            request,
            level: level.problem.level,
            input: rnl_runtime::problem::input_digest(&level.problem),
            options,
            solution: solved.solution,
            report: Some(solved.report),
        });
    }
    ResultDocument {
        solver: SolverIdentity { name: "rnl-solver".into(), version: env!("CARGO_PKG_VERSION").into() },
        levels,
    }
}

fn run(mut args: Vec<OsString>) -> Result<(), String> {
    if args == [OsString::from("--help")] || args == [OsString::from("-h")] {
        print!("{HELP}");
        return Ok(());
    }
    if args == [OsString::from("--version")] {
        println!("rnl-solver {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "solve") {
        args.remove(0);
    }
    if args.len() > 1 {
        return Err("expected at most one input file; use --help".into());
    }
    let bytes = match args.first() {
        None => {
            let mut b = Vec::new();
            std::io::stdin().read_to_end(&mut b).map_err(|e| e.to_string())?;
            b
        }
        Some(path) if path == "-" => {
            let mut b = Vec::new();
            std::io::stdin().read_to_end(&mut b).map_err(|e| e.to_string())?;
            b
        }
        Some(path) if path.to_str().is_some_and(|p| p.starts_with('-')) => {
            return Err("unknown option; use --help".into())
        }
        Some(path) => std::fs::read(path).map_err(|e| format!("{}: {e}", path.to_string_lossy()))?,
    };
    let json = rnl_solver_io::parse_json(&bytes).map_err(|e| e.to_string())?;
    let input = rnl_solver_io::decode_input(&json).map_err(|e| e.to_string())?;
    let result = rnl_solver_io::encode_result(&solve(&input)).map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &result).map_err(|e| e.to_string())?;
    writeln!(stdout).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rnl-solver: {error}");
            ExitCode::from(2)
        }
    }
}
