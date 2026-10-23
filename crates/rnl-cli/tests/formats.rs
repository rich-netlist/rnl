//! Each file of a netlist read with the adapter of its format (the plan of file formats, PR 1a).
//!
//! A frontend sets the format of a file by its path (`BuildOptions::formats`, `--format`); a file
//! with none is read as the main file is, as before. Each file's names follow its format's rules:
//! a subcircuit, a model card or a global net of one format is found from another, by case only
//! when both the reference and the definition compare names so, a definition spelt as written
//! first, more than one an error. The nets of a circuit compare by the rules of the file that
//! writes it, and `gnd` is ground only in a statement of a SPICE file. RNL is written into each
//! file with that file's comment marker.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::{FilePool, MemoryProvider, Provider};
use rnl_view::{BuildOptions, Document, FormatInfo, LibraryIndex, LibraryProvider, Parts};

fn set(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(p, f)| (p.to_string(), f.to_string())).collect()
}

fn build(entry: &str, files: &dyn Provider, formats: &[(&str, &str)]) -> Document {
    rnl_view::build(entry, files, &BuildOptions { formats: set(formats), ..BuildOptions::default() }).unwrap()
}

fn host(entry: &str, files: &dyn Provider, formats: &[(&str, &str)]) -> (HostDocument, Sink) {
    let mut sink = Sink::new();
    let doc = rnl_view::read_host_with(entry, None, &set(formats), files, &mut sink).unwrap();
    (doc, sink)
}

/// Each file of a Document, by name, with the format it was read with and its default.
fn formats_of(d: &Document) -> Vec<(&str, Option<&str>, Option<&str>)> {
    d.files.iter().map(|f| (f.name.as_str(), f.format.as_deref(), f.default_format.as_deref())).collect()
}

fn errors(d: &Document) -> Vec<&str> {
    d.diagnostics.iter().filter(|x| x.severity == "error").map(|x| x.message.as_str()).collect()
}

/// A SPICE netlist that includes a file written in Spectre's language, whose subcircuit it uses.
const TOP: &str = "* top\n.include \"cells.scs\"\nX1 a 0 AMP\nR1 a 0 1k\n.end\n";
const CELLS: &str =
    "simulator lang=spectre\nsubckt amp (p n)\nR1 (p n) resistor r=1k\n// @RNL {#R1 @PLACE(0, 0)}\nends amp\n";

#[test]
fn the_formats_are_listed_spice_first() {
    let want = [
        FormatInfo { name: "spice".into(), title: "SPICE".into(), extensions: vec![], fallback: true },
        FormatInfo { name: "spectre".into(), title: "Spectre".into(), extensions: vec!["scs".into()], fallback: false },
    ];
    assert_eq!(rnl_view::formats(), want);
    // a Document lists them too
    let d = build("/w/top.sp", &MemoryProvider::new().with("/w/top.sp", "* t\nR1 a 0 1k\n"), &[]);
    assert_eq!(d.formats, want);
}

#[test]
fn a_file_is_spectre_by_its_suffix_or_its_simulator_statement() {
    assert_eq!(rnl_view::detect_format("a/b.SCS", ""), "spectre");
    assert_eq!(rnl_view::detect_format("models.lib", "* models\nsimulator lang=spectre\n"), "spectre");
    assert_eq!(rnl_view::detect_format("top.sp", "* t\nR1 a 0 1k\n"), "spice");
    assert_eq!(rnl_view::detect_format("dir.scs/a.sp", "* t\n"), "spice");
    assert_eq!(rnl_view::detect_format(".scs", ""), "spice");
}

#[test]
fn without_a_format_set_every_file_is_read_as_the_main_file() {
    // as before: the Spectre file that a SPICE netlist includes is read as SPICE
    let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.scs", CELLS);
    let d = build("/w/top.sp", &files, &[]);
    assert_eq!(formats_of(&d), [("top.sp", Some("spice"), Some("spice")), ("cells.scs", Some("spice"), Some("spice"))]);
    assert!(!d.levels.iter().any(|l| l.title == "amp"));
}

#[test]
fn a_spice_netlist_includes_a_spectre_file() {
    let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.scs", CELLS);
    let d = build("/w/top.sp", &files, &[("/w/cells.scs", "spectre")]);
    assert_eq!(
        formats_of(&d),
        [("top.sp", Some("spice"), Some("spice")), ("cells.scs", Some("spectre"), Some("spice"))]
    );
    assert!(errors(&d).is_empty(), "{:?}", d.diagnostics);
    // Spectre's statements are read, the record in `//` too
    let amp = d.levels.iter().position(|l| l.title == "amp").expect("the subcircuit of the Spectre file");
    assert_eq!(d.files[1].records.len(), 1);
    // `AMP` of the SPICE file finds `amp` of the Spectre file: the SPICE reference ignores case
    let x1 = d.objects.iter().find(|o| o.name == "X1").unwrap();
    assert_eq!(x1.opens, Some(amp));
    // the snapshot gives each file's format, as the netlist mixes them
    let options = BuildOptions { formats: set(&[("/w/cells.scs", "spectre")]), ..BuildOptions::default() };
    let parts = Parts { drawing: false, host: false, model: false };
    let s = serde_json::to_value(rnl_view::snapshot("/w/top.sp", &files, &options, parts).unwrap()).unwrap();
    assert_eq!(s["files"][1]["format"], "spectre");
    assert_eq!(s["dialect"], "spice");
}

#[test]
fn a_snapshot_of_one_format_has_no_formats_of_files() {
    let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.scs", CELLS);
    let parts = Parts { drawing: false, host: false, model: false };
    let s = rnl_view::snapshot("/w/top.sp", &files, &BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(s).unwrap();
    assert!(s["files"].as_array().unwrap().iter().all(|f| f.get("format").is_none()), "{}", s["files"]);
}

#[test]
fn a_spectre_netlist_includes_a_spice_file() {
    // `.subckt AMP` of the SPICE file ignores case, so Spectre's `amp` finds it; at the file level,
    // `gnd` of the SPICE file is ground, that of the Spectre file is not
    let top = "// top\nsimulator lang=spectre\ninclude \"parts.inc\"\nX2 (a b) amp\nR8 (n1 gnd) resistor r=1k\n";
    let parts = "* parts\n.subckt AMP p n\nR1 p n 1k\n.ends\nR9 n1 gnd 1k\n";
    let files = MemoryProvider::new().with("/w/top.scs", top).with("/w/parts.inc", parts);
    let d = build("/w/top.scs", &files, &[("/w/parts.inc", "spice")]);
    assert_eq!(
        formats_of(&d),
        [("top.scs", Some("spectre"), Some("spectre")), ("parts.inc", Some("spice"), Some("spectre"))]
    );
    let amp = d.levels.iter().position(|l| l.title == "AMP").unwrap();
    assert_eq!(d.objects.iter().find(|o| o.name == "X2").unwrap().opens, Some(amp));
    let (doc, _) = host("/w/top.scs", &files, &[("/w/parts.inc", "spice")]);
    assert_eq!(doc.net_key_in(0, "gnd", "parts.inc"), "0");
    assert_eq!(doc.net_key_in(0, "gnd", "top.scs"), "gnd");
    // the nets of the main file's circuit compare by Spectre's rules, by case
    assert_eq!(doc.net_key_in(0, "N1", "parts.inc"), "N1");
    let names: Vec<&str> =
        d.objects.iter().filter(|o| o.kind == "net" && o.level == 0).map(|o| o.name.as_str()).collect();
    assert!(names.contains(&"0") && names.contains(&"gnd"), "{names:?}");
}

#[test]
fn three_files_of_two_formats_in_a_chain() {
    // a SPICE netlist, a Spectre file it includes, and a SPICE file that one includes
    let top = "* top\n.include \"mid.scs\"\nX0 x 0 mid\n.end\n";
    let mid = "simulator lang=spectre\ninclude \"leaf.inc\"\nsubckt mid (p n)\nX1 (p n) LEAF\nends mid\n";
    let leaf = "* leaf\n.subckt leaf p n\nR1 p n 1k\n.ends\n";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/mid.scs", mid).with("/w/leaf.inc", leaf);
    let d = build("/w/top.sp", &files, &[("/w/mid.scs", "spectre"), ("/w/leaf.inc", "spice")]);
    let shown: Vec<(&str, Option<&str>)> = formats_of(&d).into_iter().map(|(n, f, _)| (n, f)).collect();
    assert_eq!(shown, [("top.sp", Some("spice")), ("mid.scs", Some("spectre")), ("leaf.inc", Some("spice"))]);
    assert!(errors(&d).is_empty(), "{:?}", d.diagnostics);
    // Spectre's `LEAF` finds SPICE's `leaf`, whose definition ignores case
    let leaf = d.levels.iter().position(|l| l.title == "leaf").unwrap();
    assert_eq!(d.objects.iter().find(|o| o.name == "X1").unwrap().opens, Some(leaf));
}

#[test]
fn a_name_found_in_files_of_another_format_is_not_unique_when_two_are() {
    // two subcircuits of the Spectre file that SPICE's `amp` both finds, neither spelt as it is
    let top = "* top\n.include \"cells.scs\"\nX1 a 0 amp\nX2 a 0 AMP\n.end\n";
    let cells = "simulator lang=spectre\nsubckt Amp (p n)\nends Amp\nsubckt AMP (p n)\nends AMP\n";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/cells.scs", cells);
    let d = build("/w/top.sp", &files, &[("/w/cells.scs", "spectre")]);
    let found: Vec<&str> = errors(&d).into_iter().filter(|m| m.contains("not unique")).collect();
    assert_eq!(found, ["the subcircuit name amp is not unique: defined at cells.scs:2, cells.scs:4"]);
    // `AMP`, spelt as one of them, finds that one
    let x2 = d.objects.iter().find(|o| o.name == "X2").unwrap();
    assert_eq!(x2.opens.map(|l| d.levels[l].title.as_str()), Some("AMP"));
    assert_eq!(d.objects.iter().find(|o| o.name == "X1").unwrap().opens, None);
}

#[test]
fn model_cards_are_found_across_formats() {
    // SPICE's devices name Spectre's model cards, by any case; Spectre's names SPICE's card
    let top = "* top\n.include \"models.scs\"\n.include \"spice.inc\"\nM1 d g 0 0 nch\nM2 d g 0 0 Nmix\nM3 d g 0 0 nmix\n.end\n";
    let models = "simulator lang=spectre\nmodel nch bsim4 type=n\nmodel NMIX bsim4 type=n\nmodel nMix bsim4 type=p\nM4 (d g 0 0) pch_s\n";
    let spice = "* spice\n.model PCH_S pmos\n";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/models.scs", models).with("/w/spice.inc", spice);
    let (doc, sink) = host("/w/top.sp", &files, &[("/w/models.scs", "spectre")]);
    let inst = |name: &str| doc.levels[0].instances.iter().find(|i| i.name == name).unwrap();
    assert_eq!((inst("M1").mtype.as_deref(), inst("M1").model_missing), (Some("nmos"), false));
    // Spectre's `pch_s` finds SPICE's `PCH_S`, whose card ignores case
    assert_eq!(inst("M4").mtype.as_deref(), Some("pmos"));
    // `nmix` is spelt as no card is, and two are found for it; `Nmix` too
    let said: Vec<&str> =
        sink.all().iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert_eq!(
        said,
        [
            "the model name Nmix is not unique: model cards at models.scs:3, models.scs:4",
            "the model name nmix is not unique: model cards at models.scs:3, models.scs:4",
        ]
    );
    assert_eq!((inst("M2").mtype.as_deref(), inst("M3").mtype.as_deref()), (None, None));
}

#[test]
fn rnl_is_written_into_each_file_with_its_own_marker() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", TOP)
        .with("/w/cells.scs", CELLS.replace("// @RNL {#R1 @PLACE(0, 0)}\n", "").as_str());
    let (doc, _) = host("/w/top.sp", &files, &[("/w/cells.scs", "spectre")]);
    let text = |f: &str| doc.source(f).unwrap().to_string();
    assert_eq!(rnl_runtime::writeback::carrier(&doc, "top.sp", &text("top.sp")), "*");
    assert_eq!(rnl_runtime::writeback::carrier(&doc, "cells.scs", &text("cells.scs")), "//");
    // the same files read as SPICE, as before: `*` for both
    let (doc, _) = host("/w/top.sp", &files, &[]);
    assert_eq!(rnl_runtime::writeback::carrier(&doc, "cells.scs", &text("cells.scs")), "*");
}

#[test]
fn the_format_of_the_main_file_can_be_set_and_comes_before_the_dialect() {
    let files = MemoryProvider::new().with("/w/chain.sp", "// t\nr1 (a 0) resistor r=1k\n");
    let options = BuildOptions {
        dialect: Some("spice".into()),
        formats: set(&[("/w/chain.sp", "spectre")]),
        ..BuildOptions::default()
    };
    let d = rnl_view::build("/w/chain.sp", &files, &options).unwrap();
    assert_eq!((d.dialect.as_str(), formats_of(&d)[0]), ("spectre", ("chain.sp", Some("spectre"), Some("spice"))));
}

#[test]
fn an_unknown_format_fails_the_build_and_a_file_not_read_is_left_alone() {
    let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.scs", CELLS);
    let options = BuildOptions { formats: set(&[("/w/cells.scs", "verilog")]), ..BuildOptions::default() };
    let err = rnl_view::build("/w/top.sp", &files, &options).unwrap_err();
    assert_eq!(err, "unknown format \"verilog\" for /w/cells.scs; the formats are spice, spectre");
    let d = build("/w/top.sp", &files, &[("/w/elsewhere.inc", "spectre"), ("/w/./sub/../cells.scs", "spectre")]);
    assert_eq!(formats_of(&d)[1].1, Some("spectre"));
}

/// A file system that ignores case: its files under lower case paths, and its key of a path.
struct NoCase(MemoryProvider);

impl Provider for NoCase {
    fn read(&self, path: &str) -> Option<String> {
        self.0.read(&path.to_lowercase())
    }
    fn path_key(&self, path: &str) -> String {
        path.to_lowercase()
    }
}

#[test]
fn paths_compare_as_the_provider_compares_them_through_its_wrappers() {
    // the include writes `Models.SCS`, the frontend sets `/W/models.scs`; both are the file
    let top = "* top\n.include \"Models.SCS\"\nX1 a 0 amp\n.end\n";
    let files = NoCase(MemoryProvider::new().with("/w/top.sp", top).with("/w/models.scs", CELLS));
    let paths: Vec<String> = Vec::new();
    let pool = FilePool { files: &files, paths: &paths };
    let index = LibraryIndex::default();
    let library = LibraryProvider { files: &pool, index: &index, enabled: false };
    let d = build("/w/top.sp", &library, &[("/W/models.scs", "spectre")]);
    assert_eq!(formats_of(&d)[1], ("Models.SCS", Some("spectre"), Some("spice")));
    assert!(d.levels.iter().any(|l| l.title == "amp"));
}

#[test]
fn paths_compare_as_the_workspace_under_the_texts_in_memory_compares_them() {
    // edits in memory over a file system that ignores case, as the desktop layers them: the key
    // is the file system's
    let top = "* top\n.include \"Models.SCS\"\nX1 a 0 amp\n.end\n";
    let disk = NoCase(MemoryProvider::new().with("/w/models.scs", CELLS));
    let edits = MemoryProvider::new().with("/w/top.sp", top);
    let files = rnl_runtime::source::Layered { layers: vec![&edits, &disk] };
    assert_eq!(files.path_key("/W/Models.SCS"), "/w/models.scs");
    let d = build("/w/top.sp", &files, &[("/W/models.scs", "spectre")]);
    assert_eq!(formats_of(&d)[1], ("Models.SCS", Some("spectre"), Some("spice")));
}

#[test]
fn the_library_index_reads_its_files_with_their_formats() {
    // a `.scs` file written in SPICE: as Spectre, it defines nothing
    let files = MemoryProvider::new()
        .with("/top.sp", "* top\nX1 a 0 cell\n.end\n")
        .with("/lib/cells.scs", ".subckt cell p n\nR1 p n 1k\n.ends\n");
    let paths = vec!["/lib/cells.scs".to_string()];
    let mut index = LibraryIndex::default();
    index.update(&paths, &files, false);
    assert!(rnl_runtime::source::LibraryLookup::candidates(&index, "cell", false).is_empty());
    index.set_formats(set(&[("/lib/cells.scs", "spice")]));
    index.update(&paths, &files, false);
    assert_eq!(rnl_runtime::source::LibraryLookup::candidates(&index, "cell", false).len(), 1);
    // and the build that includes it automatically reads it with its format
    let library = LibraryProvider { files: &files, index: &index, enabled: true };
    let d = build("/top.sp", &library, &[("/lib/cells.scs", "spice")]);
    assert!(d.levels.iter().any(|l| l.title == "cell"), "{:?}", d.diagnostics);
    assert_eq!(formats_of(&d)[1].1, Some("spice"));
}

fn rnl(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.code().unwrap_or(-1), text(&out.stdout), text(&out.stderr))
}

#[test]
fn the_command_line_sets_formats_and_lists_them() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-formats");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("top.sp"), TOP).unwrap();
    std::fs::write(dir.join("cells.scs"), CELLS).unwrap();
    let top = rnl_cli::path_str(&dir.join("top.sp"));
    let cells = rnl_cli::path_str(&dir.join("cells.scs"));
    let (code, out, err) = rnl(&["view", &top, "--no-svg", "--format", &format!("{cells}=spectre")]);
    assert_eq!(code, 0, "{err}");
    let d: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!((&d["files"][1]["format"], &d["files"][1]["default_format"]), (&"spectre".into(), &"spice".into()));
    // without it, as before
    let (_, out, _) = rnl(&["view", &top, "--no-svg"]);
    let d: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(d["files"][1]["format"], "spice");
    // check reads with it too: the Spectre file's subcircuit is found
    let (code, out, _) = rnl(&["check", &top, "--format", &format!("{cells}=spectre")]);
    assert_eq!(code, 0, "{out}");
    let (code, _, err) = rnl(&["check", &top, "--format", &format!("{cells}=verilog")]);
    assert_eq!(
        (code, err.trim()),
        (2, format!("rnl: unknown format \"verilog\" for {cells}; the formats are spice, spectre").as_str())
    );
    let (code, out, _) = rnl(&["formats"]);
    assert_eq!(code, 0);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list, serde_json::to_value(rnl_view::formats()).unwrap());
}
