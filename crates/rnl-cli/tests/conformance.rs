//! The conformance corpus in `tests/conformance`.
//!
//! - `cases/NAME.sp` (or `.scs`) with `NAME.expected.json`: the effective model and the places of
//!   the diagnostics must match; and with `NAME.snapshot.json`, the editor interface's snapshot.
//!   Run with `RNL_BLESS=1` to write the expected files after a change that is meant.
//! - `equivalence/NAME.sp` and `NAME.scs`: the same circuit in SPICE and Spectre with the same RNL
//!   gives the same effective model (architecture 4.1, 10.2).
//! - `draft-0.3/NAME.sp` (or `.scs`): cases of the draft 0.3, read with the draft switch, with
//!   expected files of their own as in `cases`. `RNL_BLESS_DRAFT=1` writes them, so that writing
//!   them never touches the expected files of the corpus of 0.2.

use std::path::{Path, PathBuf};

use rnl_cli::{dump, interpret_file, FsProvider};
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

fn draft() -> Options {
    Options { draft: true, ..Options::default() }
}

fn run_draft(path: &Path) -> Value {
    run_with(path, &draft())
}

/// The editor interface's snapshot of a file, as `rnl snapshot` prints it, without what changes
/// with the toolkit rather than with the case: the runtime's version, the revision, which follows
/// the bundled packages' texts, and the diagnostics, which the expected file holds.
fn snapshot(path: &Path) -> Value {
    snapshot_with(path, &Options::default())
}

fn snapshot_draft(path: &Path) -> Value {
    snapshot_with(path, &draft())
}

fn snapshot_with(path: &Path, options: &Options) -> Value {
    let provider = FsProvider::new(&[]);
    let parts = rnl_view::Parts { drawing: false, host: true, model: true };
    let build = rnl_view::BuildOptions { runtime: options.clone(), ..rnl_view::BuildOptions::default() };
    let s = rnl_view::snapshot(&path.to_string_lossy(), &provider, &build, parts).unwrap();
    let mut v = serde_json::to_value(s).unwrap();
    for k in ["runtime", "revision", "diagnostics"] {
        v.as_object_mut().unwrap().remove(k);
    }
    v
}

/// Each case's `output` against `NAME.{kind}.json`, or with `RNL_BLESS=1` written there.
fn check_cases(kind: &str, what: &str, output: fn(&Path) -> Value) {
    check_cases_in("cases", "RNL_BLESS", kind, what, output)
}

/// The cases of a directory of the corpus against their expected files, or with `bless` set in
/// the environment written there.
fn check_cases_in(dir: &str, bless: &str, kind: &str, what: &str, output: fn(&Path) -> Value) {
    let dir = corpus().join(dir);
    let bless = std::env::var(bless).is_ok();
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
            .unwrap_or_else(|_| panic!("{} has no {kind} file; run with RNL_BLESS=1", f.display()));
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

#[test]
fn cases_match_their_expected_models() {
    check_cases("expected", "effective model", run);
}

#[test]
fn cases_match_their_expected_snapshots() {
    check_cases("snapshot", "snapshot", snapshot);
}

#[test]
fn draft_cases_match_their_expected_models() {
    check_cases_in("draft-0.3", "RNL_BLESS_DRAFT", "expected", "effective model of the draft 0.3", run_draft);
}

#[test]
fn draft_cases_match_their_expected_snapshots() {
    check_cases_in("draft-0.3", "RNL_BLESS_DRAFT", "snapshot", "snapshot of the draft 0.3", snapshot_draft);
}

#[test]
fn the_draft_switch_changes_nothing_in_a_file_of_0_2() {
    // the rules the draft 0.3 changes apply only to the files that declare it
    let draft = Options { draft: true, ..Options::default() };
    for dir in ["cases", "equivalence"] {
        for f in files(&corpus().join(dir), &["sp", "scs"]) {
            assert_eq!(run_with(&f, &draft), run(&f), "{}", f.display());
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

#[test]
fn spice_and_spectre_give_the_same_model() {
    let dir = corpus().join("equivalence");
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
fn standard_packages_check_clean() {
    // every version of the standard series, in a directory of its own; the series of the draft 0.3
    // is written in the language 0.3, which the toolkit reads with the draft switch until it is
    // released
    let lib = corpus().join("../../libraries/standard");
    let mut checked = 0;
    for dir in std::fs::read_dir(lib).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()) {
        let draft_series = dir.file_name().is_some_and(|n| n == rnl_runtime::interp::unit::DRAFT_VERSION);
        let options = if draft_series { draft() } else { Options::default() };
        for f in files(&dir, &["rnl"]) {
            let provider = FsProvider::new(&[]);
            let r = interpret_file(&f, None, &provider, &options).unwrap();
            let d = r.diagnostics();
            // the hints of a package checked on its own are its author's, such as the pins of the
            // gates with four inputs, which are not on integer points (9.2.4)
            let loud: Vec<String> = d
                .iter()
                .filter(|d| d.severity != rnl_runtime::diag::Severity::Hint)
                .map(|d| r.sink.render(d))
                .collect();
            assert!(loud.is_empty(), "{}: {loud:?}", f.display());
            if !draft_series {
                assert!(d.is_empty(), "{}: {:?}", f.display(), d.iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
            }
            checked += 1;
        }
    }
    assert!(checked >= 25, "only {checked} package files");
}
