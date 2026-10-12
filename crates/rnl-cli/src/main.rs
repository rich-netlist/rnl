//! `rnl`: check and draw Rich Netlist documents.

mod dump;
mod provider;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::Provider;
use rnl_runtime::{Interpretation, Options};

use provider::FsProvider;

const USAGE: &str = "usage:
  rnl check FILE [options]          print the diagnostics; exit 1 on errors
  rnl render FILE -o DIR [options]  write one SVG per drawing level
  rnl dump FILE [options]           print the effective model and diagnostics as JSON
  rnl strip-generated FILE -o OUT   write FILE without its type=\"generated\" records

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl).
options:
  --package-dir DIR   look for packages in DIR as well (16.2); repeatable
  --no-std            do not use the standard package at the root (16.3)
  --dialect NAME      spice or spectre, instead of guessing from the file";

struct Args {
    command: String,
    file: PathBuf,
    out: Option<PathBuf>,
    package_dirs: Vec<PathBuf>,
    std: bool,
    dialect: Option<String>,
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
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(it.next().ok_or("-o needs a path")?)),
            "--package-dir" => package_dirs.push(PathBuf::from(it.next().ok_or("--package-dir needs a directory")?)),
            "--no-std" => std = false,
            "--dialect" => dialect = Some(it.next().ok_or("--dialect needs spice or spectre")?),
            _ if a.starts_with('-') => return Err(format!("unknown option {a}\n{USAGE}")),
            _ => file = Some(PathBuf::from(a)),
        }
    }
    let file = file.ok_or(USAGE)?;
    Ok(Args { command, file, out, package_dirs, std, dialect })
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

/// Is the main file Spectre? By extension, or a `simulator lang=spectre` before any element.
fn is_spectre(path: &Path, text: &str, dialect: &Option<String>) -> bool {
    if let Some(d) = dialect {
        return d == "spectre";
    }
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("scs")) {
        return true;
    }
    text.lines().take(50).any(|l| {
        let l = l.trim().to_lowercase();
        l.starts_with("simulator") && l.contains("lang") && l.contains("spectre")
    })
}

fn read_document(args: &Args, provider: &dyn Provider, sink: &mut Sink) -> Result<HostDocument, String> {
    let path = provider::path_str(&args.file);
    let text = provider.read(&path).ok_or_else(|| format!("cannot read {}", args.file.display()))?;
    let doc = if is_spectre(&args.file, &text, &args.dialect) {
        rnl_host_spectre::read(&path, provider, sink)
    } else {
        rnl_host_spice::read(&path, provider, sink)
    };
    doc.ok_or_else(|| format!("cannot read {}", args.file.display()))
}

/// Read and interpret the file named on the command line.
fn interpret_file(args: &Args, provider: &FsProvider) -> Result<Interpretation, String> {
    let options = Options { std: args.std };
    let path = provider::path_str(&args.file);
    if args.file.extension().is_some_and(|e| e.eq_ignore_ascii_case("rnl")) {
        return Ok(rnl_runtime::check_package(&path, provider, &options));
    }
    let mut host_sink = Sink::new();
    let doc = read_document(args, provider, &mut host_sink)?;
    let mut result = rnl_runtime::interpret(doc, provider, &options);
    // the adapter's own diagnostics come first
    let mut sink = host_sink;
    for d in result.sink.all() {
        sink.add(d.severity, d.message.clone(), d.loc.clone(), rnl_runtime::diag::Extra { cause: None, origin: d.origin.clone(), in_package: d.in_package.clone() });
    }
    let _ = &mut result;
    Ok(Interpretation { sink, ..result })
}

fn run(args: &Args) -> Result<ExitCode, String> {
    let provider = FsProvider::new(&args.package_dirs);
    match args.command.as_str() {
        "check" => {
            let r = interpret_file(args, &provider)?;
            let shown = r.diagnostics();
            for d in &shown {
                println!("{}", r.sink.render(d));
            }
            let count = |sev| shown.iter().filter(|d| d.severity == sev).count();
            let (e, w, h) = (count(Severity::Error), count(Severity::Warning), count(Severity::Hint));
            println!("{e} errors, {w} warnings, {h} hints");
            Ok(if e > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "dump" => {
            let r = interpret_file(args, &provider)?;
            println!("{}", serde_json::to_string_pretty(&dump::interpretation(&r)).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "render" => {
            let r = interpret_file(args, &provider)?;
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
            let path = provider::path_str(&args.file);
            let text = provider.read(&path).ok_or_else(|| format!("cannot read {}", args.file.display()))?;
            let mut sink = Sink::new();
            let doc = read_document(args, &provider, &mut sink)?;
            let main = doc.main().file.clone();
            let stripped = rnl_runtime::writeback::strip_generated(&doc, &main, &text);
            std::fs::write(&out, stripped).map_err(|e| e.to_string())?;
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(format!("unknown command {}\n{USAGE}", args.command)),
    }
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}
