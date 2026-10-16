//! Reading, changing and writing back (standard 16.4, 17): host text stays as it is, and what a
//! tool writes reads back as it meant.

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, path_str, read_host, FsProvider};
use rnl_runtime::diag::Sink;
use rnl_runtime::source::{Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::{record_type, remove_records, replace_generated, strip_generated};
use rnl_runtime::Options;

fn cases() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases");
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sp"))
        .collect();
    out.sort();
    out
}

fn host_lines(text: &str) -> Vec<String> {
    text.lines().filter(|l| !l.starts_with('*')).map(String::from).collect()
}

#[test]
fn stripping_generated_records_keeps_the_rest() {
    let fs = FsProvider::new(&[]);
    for f in cases() {
        let mut sink = Sink::new();
        let doc = read_host(&f, None, &fs, &mut sink).unwrap();
        let path = path_str(&f);
        let text = fs.read(&path).unwrap();
        let main = doc.main().file.clone();
        assert_eq!(remove_records(&doc, &main, &text, |_, _| false), text, "{}", f.display());
        let stripped = strip_generated(&doc, &main, &text);
        assert_eq!(host_lines(&stripped), host_lines(&text), "{}", f.display());
        let mem = MemoryProvider::new().with(&path, &stripped);
        let both = Layered { layers: vec![&mem, &fs] };
        let mut sink = Sink::new();
        let again = read_host(&f, None, &both, &mut sink).unwrap();
        let count = |d: &rnl_runtime::host::HostDocument, generated: bool| {
            d.levels
                .iter()
                .flat_map(|l| l.records.iter())
                .filter(|r| r.file == main && (record_type(r) == "generated") == generated)
                .count()
        };
        assert_eq!(count(&again, true), 0, "{}", f.display());
        // a record removed before tokens on its line leaves an empty one, so that they still fail
        // after a record (2.4), as in 12-diagnostics
        let empty = |t: &str| t.matches("@RNL {} ").count();
        let kept = count(&doc, false) + empty(&stripped) - empty(&text);
        assert_eq!(count(&again, false), kept, "{}", f.display());
    }
}

#[test]
fn a_solver_record_written_back_reads_back() {
    let fs = FsProvider::new(&[]);
    let f = cases().into_iter().find(|p| p.ends_with("01-rc-lowpass.sp")).unwrap();
    let path = path_str(&f);
    let text = fs.read(&path).unwrap();
    let mut sink = Sink::new();
    let doc = read_host(&f, None, &fs, &mut sink).unwrap();
    let record = "@RNL(type=\"generated\", priority=\"high\", author=\"test\") {\n  #V1 @PLACE(0, 0);\n  #R1 @PLACE(6, 8) => @ORIENT(\"R90\");\n  #C1 @PLACE(12, 3)\n}";
    let written = replace_generated(&doc, 0, &text, record);
    // the record takes the file's line endings, whichever a checkout gave it
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    assert_eq!(written.matches('\n').count(), written.matches(eol).count());
    assert!(written.contains(&format!(
        "* @RNL(type=\"generated\", priority=\"high\", author=\"test\") {{{eol}*   #V1 @PLACE(0, 0);"
    )));
    assert_eq!(host_lines(&written), host_lines(&text));
    let mem = MemoryProvider::new().with(&path, &written);
    let both = Layered { layers: vec![&mem, &fs] };
    let r = interpret_file(&f, None, &both, &Options::default()).unwrap();
    let r1 = r.model.levels[0].items.iter().find(|i| i.id == "#R1").unwrap();
    assert_eq!(r1.at, (6.0, 8.0));
    assert_eq!(r1.orient.as_str(), "R90");
    let generated = r.model.levels[0].items.len();
    assert!(generated >= 3);
}
