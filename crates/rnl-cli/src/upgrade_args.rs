//! Filesystem shell for the reusable upgrade library.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rnl_cli::codec::OutputEncoding;
use rnl_cli::solve::{self, SolveRun, SolvedFile, Target};
use rnl_cli::{path_str, FsProvider};
use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::{normalize, Provider};

use crate::{Args, Said};

fn identity(path: &Path) -> Result<String, Said> {
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    let mut parent = absolute.as_path();
    let mut suffix = Vec::new();
    while !parent.exists() {
        suffix.push(parent.file_name().ok_or("invalid output path")?.to_os_string());
        parent = parent.parent().ok_or("invalid output parent")?;
    }
    let mut resolved = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    for part in suffix.iter().rev() {
        resolved.push(part);
    }
    let key = normalize(&path_str(&resolved));
    Ok(if cfg!(any(windows, target_os = "macos")) { key.to_lowercase() } else { key })
}

pub(crate) fn run(args: &Args, provider: &FsProvider) -> Result<ExitCode, Said> {
    let target = match (&args.out, args.solver.in_place) {
        (Some(dir), false) => Target::Dir(dir.clone()),
        (None, true) => Target::InPlace,
        _ => return Err("upgrade requires exactly one of -o DIR or --in-place".into()),
    };
    if !args.generated
        || args.std_version.is_some()
        || args.level.is_some()
        || args.solver.solver_options.is_some()
        || args.solver.previous
    {
        return Err("upgrade reads the complete document with its declared versions and generated records".into());
    }
    let result = upgrade(
        &path_str(&args.file),
        provider,
        &UpgradeOptions { dialect: args.dialect.clone(), formats: args.formats.clone(), std: args.std },
    )?;
    let mut files: Vec<_> =
        result.files.iter().map(|(path, text)| SolvedFile { path: path.clone(), text: text.clone() }).collect();
    // The shared writer also copies dependencies for an already upgraded entry.
    if files.is_empty() {
        files.push(SolvedFile {
            path: result.entry.clone(),
            text: provider.read(&result.entry).ok_or("cannot reread upgrade input")?,
        });
    }
    let mut run = SolveRun {
        entry: result.entry,
        levels: Vec::new(),
        not_solved: Vec::new(),
        files,
        read: result.read,
        errors: 0,
        raised: Vec::new(),
    };
    let rewritten = run.files.len();
    // Validate the complete group, including unchanged includes, before creating any output.
    if matches!(target, Target::Dir(_)) {
        for path in &run.read {
            if !run.files.iter().any(|f| &f.path == path) {
                run.files.push(SolvedFile {
                    path: path.clone(),
                    text: provider.read(path).ok_or_else(|| format!("cannot reread {path}"))?,
                });
            }
        }
    }
    let destinations = solve::all_destinations(&run, &target)?;
    run.files.truncate(rewritten);
    let report = args
        .upgrade_report
        .as_ref()
        .map(|path| -> Result<(PathBuf, String), Said> {
            if path.is_dir() {
                return Err("--report needs a file path, not a directory".into());
            }
            let key = identity(path)?;
            for other in run.read.iter().map(PathBuf::from).chain(destinations.iter().cloned()) {
                let other = identity(&other)?;
                if other == key || other.starts_with(&(key.clone() + "/")) || key.starts_with(&(other + "/")) {
                    return Err("--report must differ from every input and output file".into());
                }
            }
            let json = serde_json::to_string_pretty(&result.report).map_err(|e| e.to_string())? + "\n";
            Ok((path.clone(), json))
        })
        .transpose()?;
    let written = solve::write_with_encoding(&run, &target, OutputEncoding::Preserve);
    if let Some(error) = written.error {
        return Err(error.into());
    }
    if let Some((path, json)) = report {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, json).map_err(|e| e.to_string())?;
    }
    let changes: usize = result.report.files.iter().map(|f| f.changes.len()).sum();
    let unconverted: usize = result.report.files.iter().map(|f| f.unconverted.len()).sum();
    let errors =
        result.report.diagnostics.iter().filter(|d| matches!(d.severity.as_str(), "error" | "violation")).count();
    println!(
        "{} files written, {changes} changes, {unconverted} unconverted, {errors} target errors, {} differences",
        written.written.len() + written.copied.len(),
        result.report.differences.len()
    );
    Ok(if result.report.successful() { ExitCode::SUCCESS } else { ExitCode::from(1) })
}
