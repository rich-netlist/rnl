//! The packages bundled with the core: every `.rnl` file in a version directory of
//! `libraries/standard`, such as `0.4/rnl-std-basic.rnl`, named by its path there. The list
//! follows the directories, so a new version of the standard series, or a new package in one, is
//! bundled by adding its files.

use std::fmt::Write;
use std::path::{Path, PathBuf};

fn main() {
    let libraries = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../libraries");
    let standard = libraries.join("standard");
    println!("cargo:rerun-if-changed={}", standard.display());
    let mut files = Vec::new();
    for version in std::fs::read_dir(standard).expect("libraries/standard is read") {
        let version = version.unwrap().path();
        if version.is_dir() {
            add(&mut files, &version);
        }
    }
    files.sort();
    let mut out = String::new();
    writeln!(out, "/// The packages bundled with the core (16.2, 16.3): the standard series by their paths").unwrap();
    writeln!(out, "/// in `libraries/standard`, which `build.rs` lists.").unwrap();
    writeln!(out, "const BUNDLED: &[(&str, &str)] = &[").unwrap();
    for (name, path) in &files {
        writeln!(out, "    ({name:?}, include_str!({path:?})),").unwrap();
    }
    writeln!(out, "];").unwrap();
    let to = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundled.rs");
    std::fs::write(to, out).unwrap();
    commit();
}

/// The commit the core is built from, `RNL_BUILD_COMMIT`, which a reading of one file gives with
/// the version of the core ([`crate::FileInputs::core`]); "unknown" out of a git checkout.
fn commit() {
    let git = |args: &[&str]| -> Option<String> {
        let output = std::process::Command::new("git").args(args).output().ok()?;
        output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let commit = git(&["rev-parse", "--short=8", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=RNL_BUILD_COMMIT={commit}");
    // Worktrees keep HEAD and branch refs outside this checkout. Read the commit again after one.
    let mut paths = vec!["HEAD".to_owned(), "packed-refs".to_owned()];
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        paths.push(branch);
    }
    for path in paths {
        if let Some(path) = git(&["rev-parse", "--git-path", &path]) {
            let path = PathBuf::from(path);
            let path = if path.is_absolute() { path } else { std::env::current_dir().unwrap().join(path) };
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

/// The `.rnl` files of a directory, each named by the directory's name and its own.
fn add(files: &mut Vec<(String, String)>, dir: &Path) {
    let d = dir.file_name().unwrap().to_string_lossy().to_string();
    for file in std::fs::read_dir(dir).expect("a directory of libraries is read") {
        let file = file.unwrap().path();
        if file.extension().is_some_and(|e| e == "rnl") {
            let name = file.file_name().unwrap().to_string_lossy().to_string();
            files.push((format!("{d}/{name}"), file.to_string_lossy().replace('\\', "/")));
        }
    }
}
