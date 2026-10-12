//! The conformance corpus in `tests/conformance`.
//!
//! - `cases/NAME.sp` (or `.scs`) with `NAME.expected.json`: the effective model and the places of
//!   the diagnostics must match. Run with `RNL_BLESS=1` to write the expected files after a change
//!   that is meant.
//! - `equivalence/NAME.sp` and `NAME.scs`: the same circuit in SPICE and Spectre with the same RNL
//!   gives the same effective model (architecture 4.1, 10.2).

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
    let provider = FsProvider::new(&[]);
    let r = interpret_file(path, None, &provider, &Options::default()).unwrap();
    dump::conformance(&r)
}

#[test]
fn cases_match_their_expected_models() {
    let dir = corpus().join("cases");
    let bless = std::env::var("RNL_BLESS").is_ok();
    let mut failed = Vec::new();
    for f in files(&dir, &["sp", "scs"]) {
        let got = run(&f);
        let expected = f.with_extension("expected.json");
        let text = serde_json::to_string_pretty(&got).unwrap() + "\n";
        if bless {
            std::fs::write(&expected, &text).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&expected)
            .unwrap_or_else(|_| panic!("{} has no expected file; run with RNL_BLESS=1", f.display()));
        // compared as text: numbers are written in their shortest exact form, line endings may differ
        if want.replace("\r\n", "\n") != text {
            // the model as computed now, next to the expected one, to compare
            let _ = std::fs::write(f.with_extension("actual.json"), &text);
            failed.push(f.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(failed.is_empty(), "the effective model differs from the expected one: {failed:?}");
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
    let lib = corpus().join("../../libraries/standard");
    for f in files(&lib, &["rnl"]) {
        let provider = FsProvider::new(&[]);
        let r = interpret_file(&f, None, &provider, &Options::default()).unwrap();
        let d = r.diagnostics();
        assert!(d.is_empty(), "{}: {:?}", f.display(), d.iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
    }
}
