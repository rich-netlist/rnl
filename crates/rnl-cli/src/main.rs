//! `rnl`: check, draw and solve Rich Netlist documents.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use rnl_cli::codec::{decode, encode, OutputEncoding};
use rnl_cli::solve::{SolveOptions, Target, WallClock};
use rnl_cli::{dump, FsProvider};
use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::source::{Layered, MemoryProvider, Provider};
use rnl_runtime::{Interpretation, Options};
use rnl_solver::SolverOptions;
use rnl_view::BuildOptions;

const USAGE: &str = "usage:
  rnl --version                   print the core version and build commit
  rnl check FILE [options]          print the diagnostics; exit 1 on errors
  rnl render FILE -o DIR [options]  write one SVG per drawing level
  rnl dump FILE [options]           print the effective model and diagnostics as JSON
  rnl view FILE [options]           print the Document a viewer shows, as JSON
  rnl explain FILE REF [options]    print the usages the object REF is a member of, such as #M1, &vdd,
                                    #M5:d or %dp1, written as RNL refers to it
  rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
  rnl solve FILE -o DIR [options]   solve each drawing level and write its record of the solver back
  rnl preview FILE [options]        print the solve as the desktop viewer previews it, as JSON; writes nothing
  rnl snapshot FILE [options]       print the editor interface's snapshot, as JSON
  rnl catalog [options]             print the symbols, definitions and sets of the packages, as JSON
  rnl formats                       print the formats a file can be read with, as JSON
  rnl strip-generated FILE -o OUT [options] write FILE without its type=\"generated\" records
  rnl write REQUEST.json -o OUT     write an edit transaction's ops into its netlist

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl), which is checked on its
own; catalog and formats take none. A package file is one of the format rnl: a .rnl file, or a file
of another suffix whose first record, at the start of a line, is a header record with @PACKAGE.
A file whose suffix marks a hardware description language (.va, .vams, .v, .vh, .sv, .svh, .vhd,
.vhdl) is not a netlist: its format is text, and it is not read, unless a format is set for it.
options:
  --path DIR          search packages and bare include filenames here; repeatable
  --package-dir DIR   look for packages in DIR as well (16.2), its .rnl files; repeatable
  --no-std            do not use the standard package at the root (16.3)
  --std-version V     use rnl-std V at the root, whatever the files' language version, such as 0.2
                      to read a file of 0.3 with the series 0.2; a version pinned in a header
                      record still comes first (16.3), and V not found is an error only for the
                      files that do not pin one. rnl solve does not take it
  --no-generated      read the netlist without its type=\"generated\" records, as a solver does (2.8)
  --dialect NAME      the format of FILE, spice, spectre, rnl or text, instead of guessing from it
  --format PATH=NAME  read the file PATH with the format NAME, spice, spectre, rnl or text; repeatable.
                      PATH is relative to the current directory or absolute, as FILE may be; one
                      that the netlist does not read is warned of. A file given none is read by its own
                      suffix and first lines, a .scs file or one with simulator lang=spectre as
                      Spectre, a package file as rnl, a .va file and the like as text, and FILE by
                      --dialect before them. A file set to rnl is a package that USE-PACKAGE finds
                      by name, before those of --package-dir, whatever its suffix; FILE set to rnl
                      is checked as a package file, an error when its header has no @PACKAGE
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
  --status            give each level its solve status: fixed, unsolved, solved or stale
render options (inspection only; all disabled by default):
  --usages MODE       usage frames: semantic (no drawing of their own) or all
  --currents          show in/out terminal currents, including explicit subcircuit port symbols
  --usage-roles       show role labels on enabled usage frames
  --overlay-style FILE  JSON: palette, width, dashes, current_color; independent of RNL styles
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
  --profile OUT.json  checkpoint solver timings and counters (solve only; may affect a wall-clock deadline)
  --report            print what became of every fact, not only of the facts discarded, and the hints
                      of the facts of priority low and of the statements only they lose (8.5), whose
                      minimal conflict groups the solver then seeks all, not only the first
  a fact that has no solution on its own contradicts itself and fails (8.5): solve prints an error line
  for it, and solves the level and writes it back without it
  solve exits with 1 when an error was reported, once the files are written: one of the netlist as a
  solver reads it, whose number it gives on the standard error as rnl check --no-generated counts
  them, or one of the solver
saving options (solve and strip-generated only):
  --encoding MODE     preserve (default) or utf8; UTF-8 adds a BOM only to protect leading U+FEFF
                      copied includes that are not rewritten retain their original bytes
explain options:
  --level TITLE       look for REF only in the level with this title: a subcircuit name, or the
                      file name; otherwise in every level, each that has it in turn
  --json              print the usages as JSON
  one line for each usage: the roles the object fills · the usage · its definition · where it is
  written · where it comes from (written, a rule, a definition's body, written back by a solver) ·
  in effect, failed, overridden or disabled, with the reason; the usages that contain it follow,
  indented. A failed application, which made no usage, is listed too
snapshot options:
  --parts LIST        the parts to give, of drawing, host and model; host,model by default
catalog options (the packages are those of --package-dir and the bundled ones, each read as rnl
check reads a package file, with --no-std and --std-version):
  --package NAME      only the packages of this name; repeatable
  --all-versions      every version of each package, not only the newest
write:
  REQUEST.json is {\"dialect\", \"netlist\", \"author\", \"ops\"}, the ops as the editor interface
  draft writes them; --dialect (spice by default) and --author NAME give the fields it leaves out";

struct Args {
    overlays: rnl_view::OverlayOptions,
    command: String,
    file: PathBuf,
    out: Option<PathBuf>,
    package_dirs: Vec<PathBuf>,
    search_paths: Vec<PathBuf>,
    std: bool,
    std_version: Option<String>,
    generated: bool,
    dialect: Option<String>,
    /// `--format`: the formats set for files by path.
    formats: BTreeMap<String, String>,
    svg: bool,
    primitives: bool,
    status: bool,
    level: Option<String>,
    /// As given; `problem` and `solve` each have their own default.
    solver_options: Option<String>,
    in_place: bool,
    report: bool,
    profile: Option<PathBuf>,
    previous: bool,
    author: Option<String>,
    parts: Option<String>,
    /// `--package`: the names `catalog` lists; every name when there are none.
    packages: Vec<String>,
    all_versions: bool,
    encoding: OutputEncoding,
    /// `explain`'s REF, and `--json`.
    reference: Option<String>,
    json: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or(USAGE)?;
    if command == "-h" || command == "--help" {
        return Err(USAGE.into());
    }
    let mut positional = Vec::new();
    let mut json = false;
    let mut out = None;
    let mut package_dirs = Vec::new();
    let mut search_paths = Vec::new();
    let mut std = true;
    let mut std_version = None;
    let mut generated = true;
    let mut dialect = None;
    let mut formats = BTreeMap::new();
    let (mut svg, mut primitives, mut status) = (true, false, false);
    let mut level = None;
    let mut previous = false;
    let mut solver_options = None;
    let mut profile = None;
    let (mut in_place, mut report) = (false, false);
    let mut author = None;
    let mut parts = None;
    let mut packages = Vec::new();
    let mut all_versions = false;
    let mut overlays = rnl_view::OverlayOptions::default();
    let mut overlay_args = false;
    let mut encoding = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--usages" => {
                overlay_args = true;
                overlays.usages = match it.next().as_deref() {
                    Some("all") => rnl_view::UsageMode::All,
                    Some("semantic") => rnl_view::UsageMode::Semantic,
                    _ => return Err("--usages needs all or semantic".into()),
                };
            }
            "--currents" => {
                overlay_args = true;
                overlays.currents = true;
            }
            "--usage-roles" => {
                overlay_args = true;
                overlays.roles = true;
            }
            "--overlay-style" => {
                overlay_args = true;
                let path = it.next().ok_or("--overlay-style needs a JSON file")?;
                let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
                overlays.style = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            }
            "-o" | "--out" => out = Some(PathBuf::from(it.next().ok_or("-o needs a path")?)),
            "--package-dir" => package_dirs.push(PathBuf::from(it.next().ok_or("--package-dir needs a directory")?)),
            "--path" => {
                let dir = PathBuf::from(it.next().ok_or("--path needs a directory")?);
                package_dirs.push(dir.clone());
                search_paths.push(dir);
            }
            "--no-std" => std = false,
            "--std-version" => std_version = Some(it.next().ok_or("--std-version needs a version, such as 0.2")?),
            "--no-generated" => generated = false,
            "--dialect" => dialect = Some(it.next().ok_or("--dialect needs spice, spectre, rnl or text")?),
            "--format" => {
                let set = it.next().ok_or("--format needs PATH=NAME, such as models.inc=spectre")?;
                let (path, name) =
                    set.rsplit_once('=').ok_or("--format needs PATH=NAME, such as models.inc=spectre")?;
                formats.insert(path.to_string(), name.to_string());
            }
            "--no-svg" => svg = false,
            "--primitives" => primitives = true,
            "--status" => status = true,
            "--previous" => previous = true,
            "--level" => level = Some(it.next().ok_or("--level needs a title")?),
            "--solver-options" => solver_options = Some(it.next().ok_or("--solver-options needs a text")?),
            "--in-place" => in_place = true,
            "--report" => report = true,
            "--profile" => profile = Some(PathBuf::from(it.next().ok_or("--profile needs a JSON path")?)),
            "--author" => author = Some(it.next().ok_or("--author needs a name")?),
            "--parts" => parts = Some(it.next().ok_or("--parts needs a list, such as host,model")?),
            "--package" => packages.push(it.next().ok_or("--package needs a name, such as rnl-std-basic")?),
            "--all-versions" => all_versions = true,
            "--json" => json = true,
            "--encoding" => encoding = Some(it.next().ok_or("--encoding needs preserve or utf8")?.parse()?),
            _ if a.starts_with("--encoding=") => encoding = Some(a[11..].parse()?),
            _ if a.starts_with('-') => return Err(format!("unknown option {a}\n{USAGE}")),
            _ => positional.push(a),
        }
    }
    // explain takes FILE and REF; another command FILE, the last one given
    let reference = if command == "explain" {
        if positional.len() != 2 {
            return Err(format!("rnl explain takes FILE and REF, such as rnl explain top.sp '#M1'\n{USAGE}"));
        }
        positional.pop()
    } else {
        None
    };
    let file = positional.pop().map(PathBuf::from);
    if json && command != "explain" {
        return Err("--json is an option of explain".into());
    }
    if profile.is_some() && command != "solve" {
        return Err("--profile is an option of solve".into());
    }
    if overlay_args && command != "render" {
        return Err("overlay options are options of render".into());
    }
    overlays.validate()?;
    if encoding.is_some() && !matches!(command.as_str(), "solve" | "strip-generated") {
        return Err("--encoding is only for solve and strip-generated".into());
    }
    // the catalog reads the package directories and the bundled packages, not a file
    let file = match (command.as_str(), file) {
        ("catalog" | "formats", None) => PathBuf::new(),
        (c @ ("catalog" | "formats"), Some(f)) => {
            return Err(format!("rnl {c} takes no FILE, not {}\n{USAGE}", f.display()))
        }
        (_, file) => file.ok_or(USAGE)?,
    };
    let formats = formats.into_iter().map(|(path, name)| (format_path(&path, &file), name)).collect();
    Ok(Args {
        overlays,
        command,
        file,
        out,
        package_dirs,
        search_paths,
        std,
        std_version,
        generated,
        dialect,
        formats,
        svg,
        primitives,
        status,
        level,
        solver_options,
        in_place,
        report,
        profile,
        previous,
        author,
        parts,
        packages,
        all_versions,
        encoding: encoding.unwrap_or_default(),
        reference,
        json,
    })
}

fn main() -> ExitCode {
    let argv: Vec<_> = std::env::args().skip(1).collect();
    if argv == ["--version"] || argv == ["-V"] {
        println!("rnl {} ({})", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT);
        return ExitCode::SUCCESS;
    }
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
    Options { std: args.std, std_version: args.std_version.clone(), generated: args.generated, ..Options::default() }
}

fn interpret(args: &Args, provider: &FsProvider) -> Result<Interpretation, String> {
    rnl_cli::interpret_file_with(&args.file, args.dialect.as_deref(), &args.formats, provider, &options(args))
}

/// The path of a `--format` as the reading names the files of `file`: an include resolves against
/// the directory of `file`, relative when `file` is and absolute when it is, so the path is made
/// absolute for an absolute `file`, and relative to the current directory, when it is in it, for a
/// relative one.
fn format_path(path: &str, file: &std::path::Path) -> String {
    let p = PathBuf::from(path);
    let p = if file.is_absolute() {
        std::path::absolute(&p).unwrap_or(p)
    } else {
        // the current directory compared as the file system compares paths, by case or not
        let key = |p: &std::path::Path| rnl_cli::FsProvider::new(&[]).path_key(&rnl_cli::path_str(p));
        let cwd = std::env::current_dir().map(|d| format!("{}/", key(&d).trim_end_matches('/')));
        match cwd {
            Ok(dir) if key(&p).starts_with(&dir) => {
                let skip = rnl_cli::path_str(&p).chars().take(dir.chars().count()).map(char::len_utf8).sum::<usize>();
                PathBuf::from(&rnl_cli::path_str(&p)[skip..])
            }
            _ => p,
        }
    };
    rnl_cli::path_str(&p)
}

/// Check the `--format`s before a command reads FILE: an unknown format name is an error, and each
/// path that FILE's netlist does not read is warned of, as it sets nothing, but for one set to `rnl`,
/// a package that USE-PACKAGE finds by name. A package file FILE reads no other file.
fn check_formats(args: &Args, provider: &FsProvider) -> Result<(), String> {
    if args.formats.is_empty() || matches!(args.command.as_str(), "catalog" | "formats" | "write") {
        return Ok(());
    }
    let names: Vec<String> = rnl_view::formats().into_iter().map(|f| f.name).collect();
    if let Some((path, name)) = args.formats.iter().find(|(_, name)| !names.contains(name)) {
        return Err(format!("unknown format \"{name}\" for {path}; the formats are {}", names.join(", ")));
    }
    let format =
        rnl_view::entry_format(&args.file.to_string_lossy(), args.dialect.as_deref(), &args.formats, provider)?;
    let read: Vec<String> = if format == rnl_runtime::host::package::NAME {
        Vec::new()
    } else {
        let mut sink = Sink::new();
        let doc = rnl_view::read_host_with(
            &args.file.to_string_lossy(),
            args.dialect.as_deref(),
            &args.formats,
            provider,
            &mut sink,
        )?;
        doc.levels
            .iter()
            .filter(|l| l.kind == rnl_runtime::host::LevelKind::File)
            .map(|l| provider.path_key(&rnl_runtime::source::normalize(&l.path)))
            .collect()
    };
    for (path, name) in args.formats.iter().filter(|(_, name)| *name != rnl_runtime::host::package::NAME) {
        let key = provider.path_key(&rnl_runtime::source::normalize(path));
        if !read.contains(&key) {
            eprintln!("rnl: warning: --format {path}={name}: {} reads no such file", args.file.display());
        }
    }
    Ok(())
}

fn run(args: &Args) -> Result<ExitCode, String> {
    let provider = FsProvider::new(&args.package_dirs).with_search_paths(&args.search_paths);
    check_formats(args, &provider)?;
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
            let options = Options { std: args.std, std_version: args.std_version.clone(), ..Options::solver() };
            let (file, dialect) = (args.file.to_string_lossy(), args.dialect.as_deref());
            let (r, previous) = if args.previous {
                rnl_view::read_previous_with(&file, dialect, &args.formats, &provider, &options)?
            } else {
                (rnl_cli::interpret_file_with(&args.file, dialect, &args.formats, &provider, &options)?, Vec::new())
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
                formats: args.formats.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
                status: args.status,
            };
            let doc = rnl_view::build(&args.file.to_string_lossy(), &provider, &options)?;
            println!("{}", serde_json::to_string_pretty(&doc).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "explain" => {
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                formats: args.formats.clone(),
                runtime: options(args),
                svg: false,
                primitives: false,
                status: false,
            };
            let doc = rnl_view::build(&args.file.to_string_lossy(), &provider, &options)?;
            let reference = args.reference.as_deref().unwrap_or_default();
            let found = rnl_cli::explain::find(&doc, reference, args.level.as_deref())?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rnl_cli::explain::json(&doc, &found)).unwrap());
            } else {
                for line in rnl_cli::explain::lines(&doc, &found) {
                    println!("{line}");
                }
            }
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
                formats: args.formats.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
                status: args.status,
            };
            let snapshot = rnl_view::snapshot(&args.file.to_string_lossy(), &provider, &options, parts)?;
            println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "formats" => {
            println!("{}", serde_json::to_string_pretty(&rnl_view::formats()).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "catalog" => {
            let options = rnl_view::CatalogOptions {
                packages: (!args.packages.is_empty()).then(|| args.packages.clone()),
                all_versions: args.all_versions,
                runtime: options(args),
            };
            println!("{}", serde_json::to_string_pretty(&rnl_view::catalog(&provider, &options)).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "render" => {
            let r = interpret(args, &provider)?;
            let overlays = if args.overlays.usages != rnl_view::UsageMode::None || args.overlays.currents {
                Some(rnl_view::build(
                    &args.file.to_string_lossy(),
                    &provider,
                    &BuildOptions {
                        dialect: args.dialect.clone(),
                        formats: args.formats.clone(),
                        runtime: options(args),
                        ..BuildOptions::default()
                    },
                )?)
            } else {
                None
            };
            let out = args.out.clone().ok_or("render needs -o DIR")?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let stem = args.file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "out".into());
            for (k, lv) in r.model.levels.iter().enumerate() {
                let svg = match &overlays {
                    Some(d) => rnl_view::overlays::svg(&d.levels[k], &args.overlays)?,
                    None => rnl_render::svg(lv, &rnl_render::RenderOptions::default()),
                };
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
        "preview" => {
            // as `rnl solve` solves, and nothing written: the Document of the files as solved and the
            // report, which the desktop viewer shows (decision 0010)
            let solver = SolverOptions::default()
                .with_overrides(args.solver_options.as_deref().unwrap_or(""))
                .map_err(|e| format!("--solver-options: {e}"))?;
            let options = SolveOptions {
                dialect: args.dialect.clone(),
                formats: args.formats.clone(),
                std: args.std,
                level: args.level.clone(),
                solver,
            };
            let entry = rnl_cli::path_str(&args.file);
            let shown = rnl_cli::preview::preview(&entry, &provider, &options, &WallClock::new(), &|_| true)?;
            let out = serde_json::json!({ "document": shown.document, "report": shown.report });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "strip-generated" => {
            let out = args.out.clone().ok_or("strip-generated needs -o OUT")?;
            // Preserve the origin of the text actually read, not the adapter's display name
            // (which can be a basename). Disk wins over an exact virtual-package lookup.
            let entry = rnl_cli::path_str(&args.file);
            let (original, input) = match std::fs::read(&args.file) {
                Ok(bytes) => {
                    let text = decode(&bytes);
                    (Some(bytes), text)
                }
                Err(e) => {
                    let text = if args.file.exists() { None } else { rnl_cli::bundled().read(&entry) };
                    let text = text.ok_or_else(|| format!("cannot read {}: {e}", args.file.display()))?;
                    (None, text)
                }
            };
            // A real file can have the virtual package's relative name on Unix. The shared
            // reader reserves the <bundled> prefix, so use a physical absolute path for disk
            // sources. Do not canonicalize: relative includes retain the entered directory.
            let read_path = if original.is_some() {
                std::path::absolute(&args.file).map_err(|e| format!("cannot resolve {}: {e}", args.file.display()))?
            } else {
                args.file.clone()
            };
            // Read the captured entry, so a file changed or removed during parsing cannot change
            // which encoding belongs to its text. Includes still use the normal disk provider.
            let held = MemoryProvider::new().with(&rnl_cli::path_str(&read_path), &input);
            let files = Layered { layers: vec![&held, &provider] };
            let mut sink = Sink::new();
            let read = read_path.to_string_lossy();
            // the entry read by its absolute path, the formats' paths so too
            let formats: BTreeMap<String, String> =
                args.formats.iter().map(|(p, f)| (format_path(p, &read_path), f.clone())).collect();
            let doc = rnl_view::read_host_with(&read, args.dialect.as_deref(), &formats, &files, &mut sink)?;
            let main = doc.main().file.clone();
            let text = doc.source(&main).ok_or_else(|| format!("cannot read {}", args.file.display()))?;
            let stripped = rnl_runtime::writeback::strip_generated(&doc, &main, text);
            let bytes = encode(original.as_deref(), &stripped, args.encoding)
                .map_err(|e| format!("{}: {e}", args.file.display()))?;
            std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
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
            // the netlist is saved as OUT, so it is read under that name unless the request names one
            if let Some(name) = out.file_name().and_then(|n| n.to_str()) {
                fields.entry("file").or_insert_with(|| name.into());
            }
            let request: rnl_edit::WriteRequest =
                serde_json::from_value(request).map_err(|e| format!("the request is not valid: {e}"))?;
            std::fs::write(&out, rnl_edit::write(&request)?).map_err(|e| e.to_string())?;
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(format!("unknown command {}\n{USAGE}", args.command)),
    }
}

/// `rnl solve`: solve, print what became of each level, write the files (plan 5.10). Exits with 1
/// when any error was reported, once the files are written: an error of the netlist as a solver
/// reads it, counted as `rnl check --no-generated` counts them (7.6), or an error of the solver, a
/// fact that contradicts itself (the draft 0.3, 8.5; 7.6); with 2 when a file cannot be read or
/// written.
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
    // the groups of the facts of priority low are sought only for their hints (8.5, 7.6)
    let solver = SolverOptions { hints_for_low: args.report, ..solver };
    let options = SolveOptions {
        dialect: args.dialect.clone(),
        formats: args.formats.clone(),
        std: args.std,
        level: args.level.clone(),
        solver,
    };
    let profile_clock = args
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
    let reuse_stats = rnl_cli::state_reuse::StateReuseStats::from_env()?;
    let reuse_observer = reuse_stats.as_ref().map(|s| s.clone() as rnl_solver::profile::StateReuseHandle);
    let result = rnl_solver::profile::with_state_reuse_observer(reuse_observer, || match &profile_clock {
        Some(clock) => rnl_cli::solve::solve_with_clock(&args.file, provider, &options, clock),
        None => rnl_cli::solve::solve(&args.file, provider, &options),
    });
    if let Some(stats) = &reuse_stats {
        stats.finish()?;
    }
    if let Some(clock) = &profile_clock {
        clock.finish(result.is_ok())?;
    }
    let run = result?;
    // nothing is written when a file has no place to go
    let output_paths = rnl_cli::solve::all_destinations(&run, &target)?;
    if let Some(profile) = &args.profile {
        let profile = std::fs::canonicalize(profile).map_err(|e| format!("cannot resolve profile path: {e}"))?;
        if output_paths.iter().any(|p| std::fs::canonicalize(p).is_ok_and(|p| p == profile)) {
            return Err("--profile conflicts with a solved output path".into());
        }
    }
    println!("solver options: {}", options.solver.canonical());
    if run.levels.is_empty() && run.not_solved.is_empty() {
        println!("{} has no drawing level to solve", run.entry);
    }
    for line in rnl_cli::solve::lines(&run, args.report) {
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

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}
