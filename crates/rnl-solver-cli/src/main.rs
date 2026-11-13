use rnl_solver::{Clock, RunLimit, StopReason};
use rnl_solver_io::{InputDocument, LevelResult, ResultDocument, SolverIdentity};
use std::{
    ffi::OsString,
    io::{Read, Write},
    process::ExitCode,
    time::Instant,
};

const HELP: &str = "rnl-solver [solve] [INPUT|-]\nrnl-solver skeleton [--rounds N] [INPUT|-]\nrnl-solver compact-drawn [INPUT|-]\n\nRead a versioned RNL solver input from a file or stdin.\nWrite one solver result JSON document to stdout; diagnostics go to stderr.\nUse rnl problem --solver-input to export the input.\n\nThe experimental skeleton command compacts a grouping-tree drawing.\n--rounds adds 0 to 3 reroute/compaction rounds (default 0); kept facts remain in force.\nAll phases share the input's time or work budget.\ncompact-drawn releases authored coordinates for an inspection preview; it cannot be used for normal writeback.\n\n  --help       show this help\n  --version    show this executable's independent version\n";

#[derive(Clone, Copy)]
enum Command {
    Solve,
    Skeleton(u8),
    CompactDrawn,
}

struct Elapsed(Instant);
impl Clock for Elapsed {
    fn check_every(&self) -> u64 {
        rnl_solver::WALL_CLOCK_CHECK_EVERY
    }
    fn elapsed(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

fn observe_budget(level: usize, mut budget: rnl_solver::BudgetReport) -> (rnl_solver::BudgetReport, bool) {
    eprintln!("level {}: {} work units; limit {:?}; stopped {:?}", level, budget.work, budget.limit, budget.reason);
    let normalized =
        matches!(budget.limit, RunLimit::Time(_) | RunLimit::Work(_)) && budget.reason != StopReason::Safety;
    if normalized {
        budget.limit = RunLimit::Work(budget.work);
        if budget.reason == StopReason::Time {
            budget.reason = StopReason::Work;
        }
    }
    (budget, normalized)
}

fn compact_preview(input: &InputDocument) -> Result<serde_json::Value, String> {
    if input.levels.iter().any(|l| l.problem.version != "0.4") {
        return Err("the compaction preview requires RNL 0.4 problems".into());
    }
    let clock = Elapsed(Instant::now());
    let mut levels = Vec::new();
    for (request, level) in input.levels.iter().enumerate() {
        let made = rnl_solver::experimental::compact_drawn_with_clock(&level.problem, &level.options, &clock)?;
        let (budget, _) = observe_budget(level.problem.level, made.budget);
        levels.push(serde_json::json!({
            "request":request,"level":level.problem.level,"input":rnl_runtime::problem::input_digest(&level.problem),
            "available":made.drawing.available,"options":made.canonical_options,
            "positions":made.drawing.positions.iter().map(|p|serde_json::json!({"object":p.object,"label":p.label,"x":p.x,"y":p.y,"source_places":p.source_places.iter().map(|loc|serde_json::json!({"file":loc.file,"line":loc.line,"col":loc.col})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "relaxed_fact_indices":made.drawing.relaxed_fact_indices,"preserved_fact_indices":made.drawing.preserved_fact_indices,
            "errors":made.drawing.errors,"warnings":made.drawing.warnings,"notes":made.drawing.notes,
            "budget":rnl_solver_io::encode_budget(budget).map_err(|e|e.to_string())?,
        }));
    }
    Ok(
        serde_json::json!({"format":"rnl-skeleton-preview","version":1,"input_version":rnl_solver_io::VERSION,"solver":{"name":"rnl-solver","version":env!("CARGO_PKG_VERSION")},"levels":levels}),
    )
}

fn solve(input: &InputDocument, command: Command) -> Result<ResultDocument, String> {
    if matches!(command, Command::Skeleton(_)) && input.levels.iter().any(|l| l.problem.version != "0.4") {
        return Err("the skeleton experiment requires RNL 0.4 problems".into());
    }
    let clock = Elapsed(Instant::now());
    let mut levels = Vec::new();
    for (request, level) in input.levels.iter().enumerate() {
        let mut solved = match command {
            Command::CompactDrawn => return Err("compact-drawn returns a preview, not a solve result".into()),
            Command::Solve => rnl_solver::solve_with_clock(&level.problem, &level.options, &clock),
            Command::Skeleton(rounds) => {
                rnl_solver::experimental::solve_skeleton_with_clock(&level.problem, &level.options, rounds, &clock)?
            }
        };
        let options = solved
            .report
            .canonical_options
            .clone()
            .unwrap_or_else(|| level.options.for_problem(&level.problem).0.canonical());
        if let Some(budget) = solved.report.budget {
            let (budget, normalized) = observe_budget(level.problem.level, budget);
            solved.report.budget = Some(budget);
            if normalized {
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
    Ok(ResultDocument {
        solver: SolverIdentity { name: "rnl-solver".into(), version: env!("CARGO_PKG_VERSION").into() },
        levels,
    })
}

fn run(mut args: Vec<OsString>) -> Result<(), String> {
    if args == [OsString::from("--help")]
        || args == [OsString::from("-h")]
        || (args.len() == 2
            && ["solve", "skeleton", "compact-drawn"].iter().any(|name| args[0] == *name)
            && (args[1] == "--help" || args[1] == "-h"))
    {
        print!("{HELP}");
        return Ok(());
    }
    if args == [OsString::from("--version")] {
        println!("rnl-solver {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let command = if args.first().is_some_and(|a| a == "compact-drawn") {
        args.remove(0);
        Command::CompactDrawn
    } else if args.first().is_some_and(|a| a == "skeleton") {
        args.remove(0);
        let mut rounds = 0;
        if let Some(index) = args.iter().position(|a| a == "--rounds") {
            rounds = args
                .get(index + 1)
                .and_then(|a| a.to_str())
                .and_then(|s| s.parse::<u8>().ok())
                .filter(|n| *n <= rnl_solver::experimental::SKELETON_ROUNDS)
                .ok_or("--rounds needs an integer from 0 to 3")?;
            args.drain(index..=index + 1);
        }
        Command::Skeleton(rounds)
    } else {
        Command::Solve
    };
    if matches!(command, Command::Solve) && args.first().is_some_and(|a| a == "solve") {
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
    let result = if matches!(command, Command::CompactDrawn) {
        compact_preview(&input)?
    } else {
        rnl_solver_io::encode_result(&solve(&input, command)?).map_err(|e| e.to_string())?
    };
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
