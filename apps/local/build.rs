//! Embeds the web viewer, so that the program is one file: every file under the directory `RNL_SITE_DIR` names, or
//! `apps/web/dist` (what `node apps/web/build.mjs` writes), as an entry of the table `site.rs` in OUT_DIR, read by
//! `assets.rs`. Nothing is read at run time but what a user asks for: a program built with no site says so, and
//! `--site DIR` serves a directory instead.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=RNL_SITE_DIR");
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let site = env::var("RNL_SITE_DIR").map(PathBuf::from).unwrap_or_else(|_| manifest.join("../web/dist"));
    println!("cargo:rerun-if-changed={}", site.display());

    let mut files = Vec::new();
    if site.is_dir() {
        collect(&site, &site, &mut files);
        files.sort();
    } else {
        println!(
            "cargo:warning=no web viewer to embed: {} is not a directory; build it with `node apps/web/build.mjs` or set RNL_SITE_DIR",
            site.display()
        );
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("site.rs");
    let mut table = fs::File::create(&out).expect("site.rs can be written");
    writeln!(table, "/// The files of the web viewer: their paths below its root, and their bytes.").unwrap();
    writeln!(table, "pub static SITE: &[(&str, &[u8])] = &[").unwrap();
    for (name, path) in &files {
        // (Debug writes a string literal: backslashes of a Windows path escaped)
        writeln!(table, "    ({name:?}, include_bytes!({:?})),", path.display().to_string()).unwrap();
    }
    writeln!(table, "];").unwrap();
}

/// The files below `dir`: each as its path below `root`, `/`-separated, and its full path.
fn collect(root: &Path, dir: &Path, into: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, into);
        } else {
            let relative = path.strip_prefix(root).expect("a file below the root");
            let name = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            into.push((name, fs::canonicalize(&path).unwrap_or(path)));
        }
    }
}
