//! `rnl`: check, draw and solve Rich Netlist documents.

use std::path::PathBuf;
use std::process::ExitCode;

use rnl_cli::solve::{SolveOptions, Target};
use rnl_cli::{dump, FsProvider};
use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::{Interpretation, Options};
use rnl_solver::SolverOptions;
use rnl_view::BuildOptions;

const USAGE: &str = "usage:
  rnl check FILE [options]          print the diagnostics; exit 1 on errors
  rnl render FILE -o DIR [options]  write one SVG per drawing level
  rnl dump FILE [options]           print the effective model and diagnostics as JSON
  rnl view FILE [options]           print the Document a viewer shows, as JSON
  rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
  rnl solve FILE -o DIR [options]   solve each drawing level and write its record of the solver back
  rnl strip-generated FILE -o OUT   write FILE without its type=\"generated\" records

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl).
options:
  --package-dir DIR   look for packages in DIR as well (16.2); repeatable
  --no-std            do not use the standard package at the root (16.3)
  --no-generated      read the netlist without its type=\"generated\" records, as a solver does (2.8)
  --dialect NAME      spice or spectre, instead of guessing from the file
  --draft             for development only: read the files that declare the draft 0.3 as well
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
problem options (the levels are read without their type=\"generated\" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  the solver's options the hash covers (default: tolerance=0.000001)
solve options (the levels are read without their type=\"generated\" records; -o or --in-place is needed):
  -o DIR              write the files solved under DIR, named as they are relative to FILE's directory
  --in-place          write them over the files read instead
  --level TITLE       only the level with this title
  --solver-options S  key=value;... over the solver's defaults, for tolerance, grid, objectives, beam
                      and budget, as rnl-solver's canonical text writes them; the hash covers them
  --report            print what became of every fact, not only of the facts discarded";

struct Args {
    command: String,
    file: PathBuf,
    out: Option<PathBuf>,
    package_dirs: Vec<PathBuf>,
    std: bool,
    generated: bool,
    dialect: Option<String>,
    draft: bool,
    svg: bool,
    primitives: bool,
    level: Option<String>,
    /// As given; `problem` and `solve` each have their own default.
    solver_options: Option<String>,
    in_place: bool,
    report: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or(USAGE)?;
    if command == "-h" || command == "--help" {
        return Err(USAGE.into());
    }
    let mut file = None;
    let mut out = None;
    let mut package_dirs = Vec::new();
    let mut std = true;
    let mut generated = true;
    let mut dialect = None;
    let mut draft = false;
    let (mut svg, mut primitives) = (true, false);
    let mut level = None;
    let mut solver_options = None;
    let (mut in_place, mut report) = (false, false);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(it.next().ok_or("-o needs a path")?)),
            "--package-dir" => package_dirs.push(PathBuf::from(it.next().ok_or("--package-dir needs a directory")?)),
            "--no-std" => std = false,
            "--no-generated" => generated = false,
            "--dialect" => dialect = Some(it.next().ok_or("--dialect needs spice or spectre")?),
            "--draft" => draft = true,
            "--no-svg" => svg = false,
            "--primitives" => primitives = true,
            "--level" => level = Some(it.next().ok_or("--level needs a title")?),
            "--solver-options" => solver_options = Some(it.next().ok_or("--solver-options needs a text")?),
            "--in-place" => in_place = true,
            "--report" => report = true,
            _ if a.starts_with('-') => return Err(format!("unknown option {a}\n{USAGE}")),
            _ => file = Some(PathBuf::from(a)),
        }
    }
    let file = file.ok_or(USAGE)?;
    Ok(Args {
        command,
        file,
        out,
        package_dirs,
        std,
        generated,
        dialect,
        draft,
        svg,
        primitives,
        level,
        solver_options,
        in_place,
        report,
    })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("rnl: {msg}");
            ExitCode::from(2)
        }
    }
}

/// The runtime's options the command line gives.
fn options(args: &Args) -> Options {
    Options { std: args.std, generated: args.generated, draft: args.draft, ..Options::default() }
}

fn interpret(args: &Args, provider: &FsProvider) -> Result<Interpretation, String> {
    rnl_cli::interpret_file(&args.file, args.dialect.as_deref(), provider, &options(args))
}

fn run(args: &Args) -> Result<ExitCode, String> {
    let provider = FsProvider::new(&args.package_dirs);
    match args.command.as_str() {
        "check" => {
            let r = interpret(args, &provider)?;
            let shown = r.diagnostics();
            for d in &shown {
                println!("{}", r.sink.render(d));
            }
            let count = |sev| shown.iter().filter(|d| d.severity == sev).count();
            let (e, w, h) = (count(Severity::Error), count(Severity::Warning), count(Severity::Hint));
            for line in rnl_cli::status_lines(&r) {
                println!("{line}");
            }
            println!("{e} errors, {w} warnings, {h} hints");
            Ok(if e > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "dump" => {
            let r = interpret(args, &provider)?;
            println!("{}", serde_json::to_string_pretty(&dump::interpretation(&r)).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "problem" => {
            let problem_options = args
                .solver_options
                .clone()
                .unwrap_or_else(|| format!("tolerance={}", rnl_runtime::geom::fmt_num(rnl_runtime::geom::TOL)));
            let options = Options { std: args.std, draft: args.draft, ..Options::solver() };
            let r = rnl_cli::interpret_file(&args.file, args.dialect.as_deref(), &provider, &options)?;
            let levels: Vec<_> = r
                .problems
                .iter()
                .filter(|p| args.level.as_ref().is_none_or(|t| *t == p.title))
                .map(|p| rnl_cli::problem::level(p, &problem_options))
                .collect();
            if levels.is_empty() {
                if let Some(t) = &args.level {
                    return Err(format!("no drawing level is titled {t}"));
                }
            }
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "levels": levels })).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "view" => {
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
            };
            let doc = rnl_view::build(&args.file.to_string_lossy(), &provider, &options)?;
            println!("{}", serde_json::to_string_pretty(&doc).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "render" => {
            let r = interpret(args, &provider)?;
            let out = args.out.clone().ok_or("render needs -o DIR")?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let stem = args.file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "out".into());
            for (k, lv) in r.model.levels.iter().enumerate() {
                let svg = rnl_render::svg(lv, &rnl_render::RenderOptions::default());
                let name = if k == 0 { format!("{stem}.svg") } else { format!("{stem}.{}.svg", sanitize(&lv.title)) };
                std::fs::write(out.join(&name), svg).map_err(|e| e.to_string())?;
                println!("{}", out.join(&name).display());
            }
            let errors = r.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
            if errors > 0 {
                eprintln!("{errors} errors; run `rnl check` to see them");
            }
            Ok(ExitCode::SUCCESS)
        }
        "solve" => solve(args, &provider),
        "strip-generated" => {
            let out = args.out.clone().ok_or("strip-generated needs -o OUT")?;
            let mut sink = Sink::new();
            let doc = rnl_cli::read_host(&args.file, args.dialect.as_deref(), &provider, &mut sink)?;
            let main = doc.main().file.clone();
            // the text as the adapter read it, so a bundled package file can be stripped too
            let text = doc.source(&main).ok_or_else(|| format!("cannot read {}", args.file.display()))?;
            let stripped = rnl_runtime::writeback::strip_generated(&doc, &main, text);
            std::fs::write(&out, stripped).map_err(|e| e.to_string())?;
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(format!("unknown command {}\n{USAGE}", args.command)),
    }
}

/// `rnl solve`: solve, print what became of each level, write the files (plan 5.10).
fn solve(args: &Args, provider: &FsProvider) -> Result<ExitCode, String> {
    let target = match (&args.out, args.in_place) {
        (Some(_), true) => return Err("solve writes under -o DIR or --in-place, not both".into()),
        (Some(dir), false) => Target::Dir(dir.clone()),
        (None, true) => Target::InPlace,
        (None, false) => return Err(
            "solve needs -o DIR, to write the files solved under DIR, or --in-place, to write them over the files read"
                .into(),
        ),
    };
    let solver = SolverOptions::default()
        .with_overrides(args.solver_options.as_deref().unwrap_or(""))
        .map_err(|e| format!("--solver-options: {e}"))?;
    let options = SolveOptions {
        dialect: args.dialect.clone(),
        std: args.std,
        draft: args.draft,
        level: args.level.clone(),
        solver,
    };
    let run = rnl_cli::solve::solve(&args.file, provider, &options)?;
    // nothing is written when a file has no place to go
    rnl_cli::solve::destinations(&run, &target)?;
    println!("solver options: {}", options.solver.canonical());
    if run.levels.is_empty() {
        println!("{} has no drawing level to solve", run.entry);
    }
    for level in &run.levels {
        for line in rnl_cli::solve::report(level, args.report) {
            println!("{line}");
        }
    }
    for path in rnl_cli::solve::write(&run, &target)? {
        println!("wrote {}", rnl_cli::path_str(&path));
    }
    if run.errors > 0 {
        eprintln!("{} errors in {} as a solver reads it; run `rnl check` to see them", run.errors, run.entry);
    }
    Ok(ExitCode::SUCCESS)
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}
