//! The packages bundled with the core: every `.rnl` file in a version directory of
//! `libraries/standard`, such as `0.2/rnl-std.rnl`, named by its path there, and Analog Canvas's
//! package in `libraries/analog-canvas-extensions`, as `analog-canvas-extensions/…`. The list
//! follows the directories, so a new version of the standard series, or a new package in one, is
//! bundled by adding its files.

use std::fmt::Write;
use std::path::{Path, PathBuf};

fn main() {
    let libraries = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../libraries");
    let standard = libraries.join("standard");
    let extensions = libraries.join("analog-canvas-extensions");
    println!("cargo:rerun-if-changed={}", standard.display());
    println!("cargo:rerun-if-changed={}", extensions.display());
    let mut files = Vec::new();
    for version in std::fs::read_dir(standard).expect("libraries/standard is read") {
        let version = version.unwrap().path();
        if version.is_dir() {
            add(&mut files, &version);
        }
    }
    // not standard, in a package directory of its own (16.2)
    add(&mut files, &extensions);
    files.sort();
    let mut out = String::new();
    writeln!(out, "/// The packages bundled with the core (16.2, 16.3): the standard series by their paths").unwrap();
    writeln!(out, "/// in `libraries/standard`, and Analog Canvas's as `analog-canvas-extensions/…`, which").unwrap();
    writeln!(out, "/// `build.rs` lists.").unwrap();
    writeln!(out, "const BUNDLED: &[(&str, &str)] = &[").unwrap();
    for (name, path) in &files {
        writeln!(out, "    ({name:?}, include_str!({path:?})),").unwrap();
    }
    writeln!(out, "];").unwrap();
    let to = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundled.rs");
    std::fs::write(to, out).unwrap();
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
