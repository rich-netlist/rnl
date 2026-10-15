//! `rnl`: check and draw Rich Netlist documents.

use std::path::PathBuf;
use std::process::ExitCode;

use rnl_cli::{dump, FsProvider};
use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::{Interpretation, Options};
use rnl_view::BuildOptions;

const USAGE: &str = "usage:
  rnl check FILE [options]          print the diagnostics; exit 1 on errors
  rnl render FILE -o DIR [options]  write one SVG per drawing level
  rnl dump FILE [options]           print the effective model and diagnostics as JSON
  rnl view FILE [options]           print the Document a viewer shows, as JSON
  rnl snapshot FILE [options]       print the editor interface's snapshot, as JSON
  rnl strip-generated FILE -o OUT   write FILE without its type=\"generated\" records
  rnl write REQUEST.json -o OUT     write an edit transaction's ops into its netlist

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl).
options:
  --package-dir DIR   look for packages in DIR as well (16.2); repeatable
  --no-std            do not use the standard package at the root (16.3)
  --dialect NAME      spice or spectre, instead of guessing from the file
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
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
    dialect: Option<String>,
    svg: bool,
    primitives: bool,
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
    let mut dialect = None;
    let (mut svg, mut primitives) = (true, false);
    let mut author = None;
    let mut parts = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(it.next().ok_or("-o needs a path")?)),
            "--package-dir" => package_dirs.push(PathBuf::from(it.next().ok_or("--package-dir needs a directory")?)),
            "--no-std" => std = false,
            "--dialect" => dialect = Some(it.next().ok_or("--dialect needs spice or spectre")?),
            "--no-svg" => svg = false,
            "--primitives" => primitives = true,
            "--author" => author = Some(it.next().ok_or("--author needs a name")?),
            "--parts" => parts = Some(it.next().ok_or("--parts needs a list, such as host,model")?),
            _ if a.starts_with('-') => return Err(format!("unknown option {a}\n{USAGE}")),
            _ => file = Some(PathBuf::from(a)),
        }
    }
    let file = file.ok_or(USAGE)?;
    Ok(Args { command, file, out, package_dirs, std, dialect, svg, primitives, author, parts })
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

fn interpret(args: &Args, provider: &FsProvider) -> Result<Interpretation, String> {
    rnl_cli::interpret_file(&args.file, args.dialect.as_deref(), provider, &Options { std: args.std })
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
        "view" => {
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                runtime: Options { std: args.std },
                svg: args.svg,
                primitives: args.primitives,
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
                runtime: Options { std: args.std },
                svg: args.svg,
                primitives: args.primitives,
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

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}
