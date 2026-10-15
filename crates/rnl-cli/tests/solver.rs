//! What the runtime gives a solver (standard 2.8, 8.5, 14.8, 17): the levels read without the
//! solver's records, and the tolerance of the checks.

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;

fn case(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases").join(name)
}

#[test]
fn generated_records_are_left_out_in_every_file() {
    let fs = FsProvider::new(&[]);
    let f = case("04-hierarchy.sp");
    let with = interpret_file(&f, None, &fs, &Options::default()).unwrap();
    let without = interpret_file(&f, None, &fs, &Options { generated: false, ..Options::default() }).unwrap();
    // the main file, the subcircuit amp in it, and rdiv in the included file all lose their positions
    for title in ["04-hierarchy.sp", "amp", "rdiv"] {
        let a = with.model.levels.iter().find(|l| l.title == title).unwrap();
        let b = without.model.levels.iter().find(|l| l.title == title).unwrap();
        assert!(a.unplaced.is_empty(), "{title}: {:?}", a.unplaced);
        assert!(!b.unplaced.is_empty(), "{title}");
        assert!(b.items.len() < a.items.len(), "{title}");
    }
    let rdiv = without.model.levels.iter().find(|l| l.title == "rdiv").unwrap();
    assert_eq!(rdiv.unplaced, ["#R1", "#R2", "a usage of $IOPORT", "a usage of $IOPORT", "a usage of $IOPORT"]);
}

#[test]
fn generated_records_left_out_report_nothing() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\n* @RNL(type=\"generated\", priority=\"high\") { #R9 @PLACE(0, 0); #R1 @PLACE(1, 2)}\n.end\n";
    let files = MemoryProvider::new().with("t.sp", text);
    let options = Options { std: false, generated: false, ..Options::default() };
    let r = interpret_file(Path::new("t.sp"), None, &files, &options).unwrap();
    assert!(r.diagnostics().iter().all(|d| !d.message.contains("R9")), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].unplaced, ["#R1"]);
    // read as usual, the record places R1 and reports the unknown R9
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    assert!(r.diagnostics().iter().any(|d| d.message.contains("R9")));
    assert_eq!(r.model.levels[0].items[0].at, (1.0, 2.0));
}

#[test]
fn the_tolerance_is_an_option() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\nR2 a 0 1k\n* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(4, 0.001); {#R1 #R2} @ALIGN(\"y\")}\n.end\n";
    let files = MemoryProvider::new().with("t.sp", text);
    let status = |tolerance: f64| {
        let options = Options { std: false, tolerance, ..Options::default() };
        let r = interpret_file(Path::new("t.sp"), None, &files, &options).unwrap();
        r.model.levels[0].constraints[0].status.clone()
    };
    assert_ne!(status(rnl_runtime::geom::TOL), "ok");
    assert_eq!(status(0.01), "ok");
}
