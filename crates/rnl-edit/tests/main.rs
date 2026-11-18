//! The integration tests of the editing library, one test program: each file of this directory is a
//! module of it, so that the toolkit is linked once rather than once for each file. Run one file's
//! tests with its name as a filter, such as `cargo test -p rnl-edit --test edit junction04::`.

mod diagnostics;
mod host_diagnostics04;
mod junction04;

/// Every file of this directory is a module of this program: one that is not would not be compiled
/// nor run, with no warning.
#[test]
fn every_file_of_the_directory_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let main = std::fs::read_to_string(dir.join("main.rs")).unwrap();
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path.file_stem().and_then(|n| n.to_str()) else { continue };
        if path.extension().is_some_and(|e| e == "rs") && name != "main" && !main.contains(&format!("\nmod {name};")) {
            missing.push(name.to_string());
        }
    }
    assert!(missing.is_empty(), "files of tests/ that tests/main.rs does not declare: {missing:?}");
}
