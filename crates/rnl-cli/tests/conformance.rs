//! The conformance corpus in `tests/conformance`.
//!
//! - `cases-0.4/from-0.2` and `cases-0.4/from-0.3`, `NAME.sp` (or `.scs`) with
//!   `NAME.expected.json`: the effective model and the places of the diagnostics must match; and
//!   with `NAME.snapshot.json`, the editor interface's snapshot. The two directories are the cases
//!   written for 0.2 and for 0.3, in 0.4. Run with `RNL_BLESS_0_4=1` to write the expected files
//!   after a change that is meant.
//! - `equivalence-0.4/NAME.sp` and `NAME.scs`: the same circuit in SPICE and Spectre with the same
//!   RNL gives the same effective model (architecture 4.1, 10.2).

use std::path::{Path, PathBuf};

use rnl_cli::{dump, interpret_file, FsProvider};
use rnl_runtime::interp::unit::scan_language_decl;
use rnl_runtime::Options;
use serde_json::Value;

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

fn files(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.contains(&e)))
        .collect();
    out.sort();
    out
}

fn run(path: &Path) -> Value {
    run_with(path, &Options::default())
}

fn run_with(path: &Path, options: &Options) -> Value {
    let provider = FsProvider::new(&[]);
    let r = interpret_file(path, None, &provider, options).unwrap();
    dump::conformance(&r)
}

/// The editor interface's snapshot of a file, as `rnl snapshot` prints it, without what changes
/// with the toolkit rather than with the case: the runtime's version, the revision, which follows
/// the bundled packages' texts, and the diagnostics, which the expected file holds.
fn snapshot(path: &Path) -> Value {
    let provider = FsProvider::new(&[]);
    let parts = rnl_view::Parts { drawing: false, host: true, model: true };
    let build = rnl_view::BuildOptions::default();
    let s = rnl_view::snapshot(&path.to_string_lossy(), &provider, &build, parts).unwrap();
    let mut v = serde_json::to_value(s).unwrap();
    for k in ["runtime", "revision", "diagnostics"] {
        v.as_object_mut().unwrap().remove(k);
    }
    v
}

/// The cases of a directory of the corpus against their expected files, or with `variable` set in
/// the environment written there.
fn check_cases_in(dir: &str, variable: &str, kind: &str, what: &str, output: fn(&Path) -> Value) {
    let dir = corpus().join(dir);
    let bless = std::env::var(variable).is_ok();
    let mut failed = Vec::new();
    for f in files(&dir, &["sp", "scs"]) {
        let got = output(&f);
        let expected = f.with_extension(format!("{kind}.json"));
        let text = serde_json::to_string_pretty(&got).unwrap() + "\n";
        if bless {
            std::fs::write(&expected, &text).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&expected)
            .unwrap_or_else(|_| panic!("{} has no {kind} file; run with {variable}=1", f.display()));
        // compared as text: numbers are written in their shortest exact form, line endings may differ
        if want.replace("\r\n", "\n") != text {
            // the output as computed now, next to the expected one, to compare
            let actual = if kind == "expected" { "actual".to_string() } else { format!("actual-{kind}") };
            let _ = std::fs::write(f.with_extension(format!("{actual}.json")), &text);
            failed.push(f.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(failed.is_empty(), "the {what} differs from the expected one: {failed:?}");
}

/// The cases, in two families by the version they were first written for, with cases of the
/// versions that are not read among them.
const MODERN_CASES: [&str; 2] = ["cases-0.4/from-0.2", "cases-0.4/from-0.3"];

#[test]
fn cases_of_0_4_match_their_expected_models() {
    for dir in MODERN_CASES {
        check_cases_in(dir, "RNL_BLESS_0_4", "expected", "effective model of 0.4", run);
    }
}

#[test]
fn cases_of_0_4_match_their_expected_snapshots() {
    for dir in MODERN_CASES {
        check_cases_in(dir, "RNL_BLESS_0_4", "snapshot", "snapshot of 0.4", snapshot);
    }
}

#[test]
fn migrated_cases_declare_the_modern_language_except_the_unsupported_version_test() {
    for dir in MODERN_CASES.into_iter().chain(["equivalence-0.4"]) {
        for file in files(&corpus().join(dir), &["sp", "scs"]) {
            let source = std::fs::read_to_string(&file).unwrap();
            let expected = if file.file_name().unwrap() == "13-unsupported-version.sp" { "9.0" } else { "0.4" };
            assert_eq!(scan_language_decl(&source).as_deref(), Some(expected), "{}", file.display());
        }
    }
}

/// Remove what names the file: the dialects' files differ only in that.
fn without_files(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("file");
            m.remove("col");
            m.remove("title");

            for (_, x) in m.iter_mut() {
                without_files(x);
            }
        }
        Value::Array(a) => {
            for x in a {
                without_files(x);
            }
        }
        _ => {}
    }
}

fn check_equivalence(directory: &str) {
    let dir = corpus().join(directory);
    let pairs = files(&dir, &["sp"]);
    assert!(!pairs.is_empty());
    for sp in pairs {
        let scs = sp.with_extension("scs");
        let mut a = run(&sp);
        let mut b = run(&scs);
        without_files(&mut a);
        without_files(&mut b);
        assert_eq!(a, b, "{} and its Spectre version differ", sp.display());
        assert!(!a["levels"][0]["items"].as_array().unwrap().is_empty());
    }
}

#[test]
fn modern_spice_and_spectre_give_the_same_model() {
    check_equivalence("equivalence-0.4");
}

/// The package files of the standard series, a version in a directory of its own: the series 0.4.
fn standard_packages() -> Vec<PathBuf> {
    let lib = corpus().join("../../libraries/standard");
    let mut out = Vec::new();
    for dir in std::fs::read_dir(lib).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()) {
        out.extend(files(&dir, &["rnl"]));
    }
    out
}

/// The series 0.4 is fully understood, including its aliases and header rules: not even a hint,
/// such as one for a pin off the integer points (9.2.4).
#[test]
fn the_formal_standard_series_checks_clean() {
    let packages = standard_packages();
    assert_eq!(packages.len(), 16, "expected 16 package files, got {}", packages.len());
    for file in packages {
        let read = interpret_file(&file, None, &FsProvider::new(&[]), &Options::default()).unwrap();
        let diagnostics: Vec<_> = read.diagnostics().iter().map(|d| read.sink.render(d)).collect();
        assert!(diagnostics.is_empty(), "{}: {diagnostics:?}", file.display());
    }
}
