//! The standard packages bundled with the core: every `.rnl` file in a version directory of
//! `libraries/standard`, such as `0.2/rnl-std.rnl`, named by its path there. The list follows the
//! directory, so a new version of the standard series, or a new package in one, is bundled by
//! adding its files.

use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../libraries/standard");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files = Vec::new();
    for version in std::fs::read_dir(dir).expect("libraries/standard is read") {
        let version = version.unwrap().path();
        if !version.is_dir() {
            continue;
        }
        let v = version.file_name().unwrap().to_string_lossy().to_string();
        for file in std::fs::read_dir(version).unwrap() {
            let file = file.unwrap().path();
            if file.extension().is_some_and(|e| e == "rnl") {
                let name = file.file_name().unwrap().to_string_lossy().to_string();
                files.push((format!("{v}/{name}"), file.to_string_lossy().replace('\\', "/")));
            }
        }
    }
    files.sort();
    let mut out = String::new();
    writeln!(out, "/// The standard packages bundled with the core (16.2, 16.3), by their paths in").unwrap();
    writeln!(out, "/// `libraries/standard`, which `build.rs` lists.").unwrap();
    writeln!(out, "const BUNDLED: &[(&str, &str)] = &[").unwrap();
    for (name, path) in &files {
        writeln!(out, "    ({name:?}, include_str!({path:?})),").unwrap();
    }
    writeln!(out, "];").unwrap();
    let to = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundled.rs");
    std::fs::write(to, out).unwrap();
}
