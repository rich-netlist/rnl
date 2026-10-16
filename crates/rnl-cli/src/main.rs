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
  rnl snapshot FILE [options]       print the editor interface's snapshot, as JSON
  rnl strip-generated FILE -o OUT   write FILE without its type=\"generated\" records
  rnl write REQUEST.json -o OUT     write an edit transaction's ops into its netlist

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
  --status            give each level its solve status: fixed, unsolved, solved or stale
problem options (the levels are read without their type=\"generated\" records):
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
  --report            print what became of every fact, not only of the facts discarded, and the hints
                      of the facts of priority low and of the statements only they lose (8.5)
snapshot options:
  --parts LIST        the parts to give, of drawing, host and model; host,model by default
write:
  REQUEST.json is {\"dialect\", \"netlist\", \"author\", \"ops\"}, the ops as the editor interface
  draft writes them; --dialect (spice by default) and --author NAME give the fields it leaves out";

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
    status: bool,
    level: Option<String>,
    /// As given; `problem` and `solve` each have their own default.
    solver_options: Option<String>,
    in_place: bool,
    report: bool,
    previous: bool,
    author: Option<String>,
    parts: Option<String>,
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
    let (mut svg, mut primitives, mut status) = (true, false, false);
    let mut level = None;
    let mut previous = false;
    let mut solver_options = None;
    let (mut in_place, mut report) = (false, false);
    let mut author = None;
    let mut parts = None;
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
            "--status" => status = true,
            "--previous" => previous = true,
            "--level" => level = Some(it.next().ok_or("--level needs a title")?),
            "--solver-options" => solver_options = Some(it.next().ok_or("--solver-options needs a text")?),
            "--in-place" => in_place = true,
            "--report" => report = true,
            "--author" => author = Some(it.next().ok_or("--author needs a name")?),
            "--parts" => parts = Some(it.next().ok_or("--parts needs a list, such as host,model")?),
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
        status,
        level,
        solver_options,
        in_place,
        report,
        previous,
        author,
        parts,
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
            // the options as the solver's canonical text writes them, so that the hash is the one
            // `rnl solve` writes with the same options
            let problem_options = SolverOptions::default()
                .with_overrides(args.solver_options.as_deref().unwrap_or(""))
                .map_err(|e| format!("--solver-options: {e}"))?
                .canonical();
            let options = Options { std: args.std, draft: args.draft, ..Options::solver() };
            let (r, previous) = if args.previous {
                rnl_view::read_previous(&args.file.to_string_lossy(), args.dialect.as_deref(), &provider, &options)?
            } else {
                (rnl_cli::interpret_file(&args.file, args.dialect.as_deref(), &provider, &options)?, Vec::new())
            };
            let levels: Vec<_> = r
                .problems
                .iter()
                .enumerate()
                .filter(|(_, p)| args.level.as_ref().is_none_or(|t| *t == p.title))
                .map(|(k, p)| {
                    let mut level = rnl_cli::problem::level(p, &problem_options);
                    if args.previous {
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
        "view" => {
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
                status: args.status,
            };
            let doc = rnl_view::build(&args.file.to_string_lossy(), &provider, &options)?;
            println!("{}", serde_json::to_string_pretty(&doc).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "snapshot" => {
            let list = args.parts.as_deref().unwrap_or("host,model");
            let mut parts = rnl_view::Parts { drawing: false, host: false, model: false };
            for p in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                match p {
                    "drawing" => parts.drawing = true,
                    "host" => parts.host = true,
                    "model" => parts.model = true,
                    _ => return Err(format!("--parts: unknown part {p}; the parts are drawing, host and model")),
                }
            }
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
                status: args.status,
            };
            let snapshot = rnl_view::snapshot(&args.file.to_string_lossy(), &provider, &options, parts)?;
            println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
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
        "write" => {
            let out = args.out.clone().ok_or("write needs -o OUT")?;
            let text =
                std::fs::read_to_string(&args.file).map_err(|e| format!("cannot read {}: {e}", args.file.display()))?;
            let mut request: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", args.file.display()))?;
            let fields = request.as_object_mut().ok_or("the request is not a JSON object")?;
            if let Some(d) = &args.dialect {
                fields.insert("dialect".into(), d.clone().into());
            }
            if let Some(a) = &args.author {
                fields.insert("author".into(), a.clone().into());
            }
            fields.entry("dialect").or_insert_with(|| "spice".into());
            let request: rnl_edit::WriteRequest =
                serde_json::from_value(request).map_err(|e| format!("the request is not valid: {e}"))?;
            std::fs::write(&out, rnl_edit::write(&request)?).map_err(|e| e.to_string())?;
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
    if run.levels.is_empty() && run.not_solved.is_empty() {
        println!("{} has no drawing level to solve", run.entry);
    }
    for line in rnl_cli::solve::lines(&run, args.report) {
        println!("{line}");
    }
    let writing = rnl_cli::solve::write(&run, &target);
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
        eprintln!("{} errors in {} as a solver reads it; run `rnl check` to see them", run.errors, run.entry);
    }
    match writing.error {
        Some(e) => Err(e),
        None => Ok(ExitCode::SUCCESS),
    }
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}
