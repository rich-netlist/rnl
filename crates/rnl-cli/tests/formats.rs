//! Each file of a netlist read with the adapter of its format (the plan of file formats, PR 1a and
//! 1b).
//!
//! A frontend sets the format of a file by its path (`BuildOptions::formats`, `--format`); a file
//! with none is read by its own suffix and first lines (`detect_format`), the main file by
//! `dialect` before them. Each file's names follow its format's rules:
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
        FormatInfo {
            name: "text".into(),
            title: "Text".into(),
            extensions: ["va", "vams", "v", "vh", "sv", "svh", "vhd", "vhdl"].map(String::from).to_vec(),
            fallback: false,
        },
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
fn without_a_format_set_each_file_is_read_by_its_own() {
    // the `.scs` file that a SPICE netlist includes is Spectre, by its suffix (plan 4.1)
    let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.scs", CELLS);
    let d = build("/w/top.sp", &files, &[]);
    assert_eq!(
        formats_of(&d),
        [("top.sp", Some("spice"), Some("spice")), ("cells.scs", Some("spectre"), Some("spectre"))]
    );
    assert!(d.levels.iter().any(|l| l.title == "amp"));
    assert!(errors(&d).is_empty(), "{:?}", d.diagnostics);
}

#[test]
fn a_model_library_is_spectre_by_its_simulator_statement() {
    // as the PDKs write their model libraries: not `.scs`, and `simulator lang=spectre` at the top
    let top = "* top\n.include \"models.lib\"\nM1 d g 0 0 nch\n.end\n";
    let lib = "* models\nsimulator lang=spectre\nmodel nch bsim4 type=n\n";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/models.lib", lib);
    let (doc, _) = host("/w/top.sp", &files, &[]);
    let m1 = &doc.levels[0].instances[0];
    assert_eq!((m1.mtype.as_deref(), m1.model_missing), (Some("nmos"), false));
    assert_eq!(doc.file_format("models.lib").map(|f| f.format.as_str()), Some("spectre"));
}

#[test]
fn the_spice_file_a_spectre_netlist_includes_is_read_as_spice() {
    // plan 4.2: the names of its subcircuit ignore case, `gnd` is ground, its elements name no
    // Spectre master, and a `simulator lang=spectre` after its first 50 lines switches nothing
    let filler = "* filler\n".repeat(50);
    let top = "// top\nsimulator lang=spectre\ninclude \"parts.inc\"\n";
    let parts = format!(
        "* parts\n.subckt sp a b\nR1 a gnd 1k\nr1 a b 1k\n.ends\n{filler}simulator lang=spectre\n// @RNL {{ #R1 @PLACE(0, 0) }}\n"
    );
    let files = MemoryProvider::new().with("/w/top.scs", top).with("/w/parts.inc", &parts);
    let (doc, sink) = host("/w/top.scs", &files, &[]);
    assert_eq!(doc.file_format("parts.inc").map(|f| f.format.as_str()), Some("spice"));
    let said: Vec<&str> = sink.all().iter().map(|d| d.message.as_str()).collect();
    assert!(said.contains(&"element r1 is defined twice"), "{said:?}");
    let sp = doc.levels.iter().position(|l| l.name == "sp").unwrap();
    let r1 = &doc.levels[sp].instances[0];
    assert_eq!((r1.model.as_deref(), doc.net_key_in(sp, "GND", "parts.inc")), (None, "0".to_string()));
    // `//` is no comment of SPICE's, so no record is read after the switch
    assert!(doc.levels.iter().all(|l| l.records.is_empty()));
}

#[test]
fn the_dialect_names_the_main_files_format_only() {
    // plan 4.4: `--dialect spectre` reads top.sp as Spectre, and the file it includes by its own
    let files = MemoryProvider::new()
        .with("/w/top.sp", "// t\nsimulator lang=spectre\ninclude \"a.inc\"\nr1 (a 0) resistor r=1k\n")
        .with("/w/a.inc", "* a\nR2 a 0 1k\n");
    let options = BuildOptions { dialect: Some("spectre".into()), ..BuildOptions::default() };
    let d = rnl_view::build("/w/top.sp", &files, &options).unwrap();
    assert_eq!(formats_of(&d), [("top.sp", Some("spectre"), Some("spectre")), ("a.inc", Some("spice"), Some("spice"))]);
    // and a format set for it reads it as Spectre, as the dialect used to
    let options = BuildOptions { formats: set(&[("/w/a.inc", "spectre")]), ..options };
    let d = rnl_view::build("/w/top.sp", &files, &options).unwrap();
    assert_eq!(formats_of(&d)[1], ("a.inc", Some("spectre"), Some("spice")));
}

#[test]
fn a_spice_netlist_includes_a_spectre_file() {
    let files = MemoryProvider::new().with("/w/top.sp", TOP).with("/w/cells.scs", CELLS);
    let d = build("/w/top.sp", &files, &[("/w/cells.scs", "spectre")]);
    assert_eq!(
        formats_of(&d),
        [("top.sp", Some("spice"), Some("spice")), ("cells.scs", Some("spectre"), Some("spectre"))]
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
    let top = "* top\n.include \"parts.inc\"\nX1 a 0 amp\n.end\n";
    let files =
        MemoryProvider::new().with("/w/top.sp", top).with("/w/parts.inc", ".subckt amp p n\nR1 p n 1k\n.ends\n");
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
    let parts = "* parts\n.subckt AMP p n\nR1 p n 1k\n.ends\nR9 n1 gnd 1k\nI5 n1 0 1m\n";
    let files = MemoryProvider::new().with("/w/top.scs", top).with("/w/parts.inc", parts);
    let d = build("/w/top.scs", &files, &[("/w/parts.inc", "spice")]);
    assert_eq!(
        formats_of(&d),
        [("top.scs", Some("spectre"), Some("spectre")), ("parts.inc", Some("spice"), Some("spice"))]
    );
    let amp = d.levels.iter().position(|l| l.title == "AMP").unwrap();
    assert_eq!(d.objects.iter().find(|o| o.name == "X2").unwrap().opens, Some(amp));
    let (doc, _) = host("/w/top.scs", &files, &[("/w/parts.inc", "spice")]);
    // the SPICE file's elements are SPICE's: no Spectre master for a current source (10.1)
    let i5 = doc.levels[0].instances.iter().find(|i| i.name == "I5").unwrap();
    assert_eq!((i5.model.as_deref(), i5.letter), (None, Some('I')));
    let r8 = doc.levels[0].instances.iter().find(|i| i.name == "R8").unwrap();
    assert_eq!(r8.model.as_deref(), Some("resistor"));
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
fn subcircuits_one_format_tells_apart_keep_their_own_block_symbols() {
    // `Amp` and `amp` of a Spectre file, under a SPICE netlist whose own names ignore case
    let top = "* top\n.include \"cells.scs\"\nX1 a 0 Amp\nX2 b 0 amp\n.end\n";
    let cells = "simulator lang=spectre\nsubckt Amp (p n)\n// @RNL { # @SET-SYMBOL $RESISTOR }\nends Amp\n\
                 subckt amp (p n)\n// @RNL { # @SET-SYMBOL $CAPACITOR }\nends amp\n";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/cells.scs", cells);
    let options = BuildOptions { formats: set(&[("/w/cells.scs", "spectre")]), ..BuildOptions::default() };
    let parts = Parts { drawing: false, host: false, model: true };
    let s = serde_json::to_value(rnl_view::snapshot("/w/top.sp", &files, &options, parts).unwrap()).unwrap();
    let model = &s["levels"][0]["model"];
    let symbol = |id: &str| {
        let object = model["objects"].as_array().unwrap().iter().find(|o| o["id"] == id).unwrap();
        let symbol = &object["symbol"]["value"];
        let found = model["symbols"].as_array().unwrap().iter().find(|x| &x["id"] == symbol).unwrap();
        found["name"].as_str().unwrap().to_string()
    };
    assert_eq!((symbol("#X1"), symbol("#X2")), ("$RESISTOR".to_string(), "$CAPACITOR".to_string()));
}

#[test]
fn a_model_name_that_is_not_unique_makes_no_subcircuit() {
    // two cards found without case, `Nch` of the Spectre file's SPICE mode and `NCH` of the SPICE
    // file, neither spelt as the Spectre instance writes it
    let top = "* top\n.include \"models.scs\"\n.model NCH nmos\n.end\n";
    let models = "simulator lang=spice\n.model Nch nmos\nsimulator lang=spectre\nM9 (d g 0 0) nch\n";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/models.scs", models);
    let (doc, sink) = host("/w/top.sp", &files, &[("/w/models.scs", "spectre")]);
    let m9 = doc.levels[0].instances.iter().find(|i| i.name == "M9").unwrap();
    assert_eq!((m9.is_subckt, m9.mtype.as_deref()), (false, None));
    let said: Vec<&str> = sink.all().iter().map(|d| d.message.as_str()).collect();
    assert!(said.contains(&"the model name nch is not unique: model cards at models.scs:2, top.sp:3"), "{said:?}");
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
    // as its format by default; read as SPICE, `*` for both
    let (doc, _) = host("/w/top.sp", &files, &[]);
    assert_eq!(rnl_runtime::writeback::carrier(&doc, "cells.scs", &text("cells.scs")), "//");
    let (doc, _) = host("/w/top.sp", &files, &[("/w/cells.scs", "spice")]);
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
    assert_eq!(err, "unknown format \"verilog\" for /w/cells.scs; the formats are spice, spectre, text");
    let d = build("/w/top.sp", &files, &[("/w/elsewhere.inc", "spectre"), ("/w/./sub/../cells.scs", "spice")]);
    assert_eq!(formats_of(&d)[1].1, Some("spice"));
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
    let d = build("/w/top.sp", &library, &[("/W/models.scs", "spice")]);
    assert_eq!(formats_of(&d)[1], ("Models.SCS", Some("spice"), Some("spectre")));
    assert!(!d.levels.iter().any(|l| l.title == "amp"));
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
    let d = build("/w/top.sp", &files, &[("/W/models.scs", "spice")]);
    assert_eq!(formats_of(&d)[1], ("Models.SCS", Some("spice"), Some("spectre")));
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

#[test]
fn auto_include_adds_no_library_that_would_change_what_a_reference_finds() {
    // the review of PR 1a: a Spectre netlist's `amp`, which the SPICE file's `AMP` finds; a library
    // of `foo`, which that file also needs, and of `AMP`, which would take `AMP` from `amp`
    let top = "// top\nsimulator lang=spectre\nsubckt amp (p n)\nends amp\ninclude \"parts.inc\"\n";
    let parts = "* parts\nX3 a b AMP\nX4 a b foo\n";
    let lib = "* cells\n.subckt foo p n\n.ends\n.subckt AMP p n\n.ends\n";
    let files = MemoryProvider::new().with("/w/top.scs", top).with("/w/parts.inc", parts).with("/lib/cells.inc", lib);
    let paths = vec!["/lib/cells.inc".to_string()];
    let mut index = LibraryIndex::default();
    assert!(index.update(&paths, &files, false).is_empty());
    let library = LibraryProvider { files: &files, index: &index, enabled: true };
    let d = build("/w/top.scs", &library, &[]);
    assert!(d.files.iter().all(|f| f.name != "/lib/cells.inc"), "{:?}", formats_of(&d));
    let amp = d.levels.iter().position(|l| l.title == "amp").unwrap();
    assert_eq!(d.objects.iter().find(|o| o.name == "X3").unwrap().opens, Some(amp));
    let warned = d.diagnostics.iter().any(|x| x.message.contains("was not included for subcircuit foo"));
    assert!(warned, "{:?}", d.diagnostics);
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
    let (code, out, err) = rnl(&["view", &top, "--no-svg", "--format", &format!("{cells}=spice")]);
    assert_eq!(code, 0, "{err}");
    let d: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!((&d["files"][1]["format"], &d["files"][1]["default_format"]), (&"spice".into(), &"spectre".into()));
    // without it, by its own
    let (_, out, _) = rnl(&["view", &top, "--no-svg"]);
    let d: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(d["files"][1]["format"], "spectre");
    // check reads with it too: the Spectre file's subcircuit is found
    let (code, out, _) = rnl(&["check", &top, "--format", &format!("{cells}=spectre")]);
    assert_eq!(code, 0, "{out}");
    let (code, _, err) = rnl(&["check", &top, "--format", &format!("{cells}=verilog")]);
    assert_eq!(
        (code, err.trim()),
        (2, format!("rnl: unknown format \"verilog\" for {cells}; the formats are spice, spectre, text").as_str())
    );
    // a path the netlist does not read sets nothing, and is warned of, by its path made absolute
    // as FILE is
    let (_, _, err) = rnl(&["check", &top, "--format", "nowhere.inc=spectre"]);
    assert!(err.contains("nowhere.inc=spectre:") && err.contains("reads no such file"), "{err}");
    // a package file checks the names too
    let package = rnl_cli::path_str(&dir.join("p.rnl"));
    std::fs::write(&package, "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"p\", version=\"1.0\")}\n")
        .unwrap();
    let (code, _, err) = rnl(&["check", &package, "--format", "x.inc=bogus"]);
    assert_eq!((code, err.contains("unknown format \"bogus\"")), (2, true), "{err}");
    let (code, out, _) = rnl(&["formats"]);
    assert_eq!(code, 0);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list, serde_json::to_value(rnl_view::formats()).unwrap());
}

/// A Verilog-A model, which SPICE would read as elements outside its subset.
const MODEL_VA: &str = "`include \"disciplines.vams\"\n// @RNL { (0,0) @TEXT(\"not a record\") }\n\
                        module res(p, n);\n  inout p, n;\n  electrical p, n;\n  parameter real r = 1k;\n\
                        analog begin\n    V(p, n) <+ r * I(p, n);\n  end\nendmodule\n";

/// Each diagnostic of a Document: its severity, file, line and message.
fn said(d: &Document) -> Vec<(&str, &str, u32, &str)> {
    d.diagnostics.iter().map(|x| (x.severity, x.file.as_str(), x.line, x.message.as_str())).collect()
}

#[test]
fn a_file_of_a_hardware_description_language_is_text() {
    for (path, language) in [
        ("m/res.va", "Verilog-A"),
        ("RES.VA", "Verilog-A"),
        ("res.vams", "Verilog-AMS"),
        ("top.v", "Verilog"),
        ("defs.vh", "Verilog"),
        ("top.sv", "SystemVerilog"),
        ("defs.svh", "SystemVerilog"),
        ("top.vhd", "VHDL"),
        ("top.vhdl", "VHDL"),
    ] {
        assert_eq!(rnl_view::detect_format(path, "* t\nR1 a 0 1k\n"), "text", "{path}");
        assert_eq!(rnl_runtime::host::text::language(path), Some(language), "{path}");
    }
    // by its suffix only: a suffix alone, or one of a directory, is not one
    for path in [".va", "va", "dir.va/top.sp", "top.vams.sp", "top.vo"] {
        assert_eq!(rnl_view::detect_format(path, "* t\n"), "spice", "{path}");
    }
}

#[test]
fn a_verilog_a_file_opened_is_not_read_and_says_so_once() {
    let files = MemoryProvider::new().with("/w/res.va", MODEL_VA);
    let d = build("/w/res.va", &files, &[]);
    assert_eq!((d.dialect.as_str(), formats_of(&d)), ("text", vec![("res.va", Some("text"), Some("text"))]));
    // one hint, at its first line: no SPICE warnings, no hint of a missing header record
    assert_eq!(said(&d), [("hint", "res.va", 1, "`res.va` is not a netlist (Verilog-A); it is not read")]);
    // its text is kept, nothing of it read: no element, no record
    assert_eq!(d.files[0].text, MODEL_VA);
    assert!(d.files[0].records.is_empty());
    assert_eq!(d.levels.len(), 1);
    assert!(d.objects.is_empty(), "{:?}", d.objects.iter().map(|o| &o.name).collect::<Vec<_>>());
    assert!(d.levels[0].unplaced.is_empty());
    // the snapshot names its format too
    let parts = Parts { drawing: false, host: false, model: false };
    let s = serde_json::to_value(rnl_view::snapshot("/w/res.va", &files, &BuildOptions::default(), parts).unwrap())
        .unwrap();
    assert_eq!(s["dialect"], "text");
}

#[test]
fn a_format_given_for_a_verilog_a_file_reads_it() {
    let files = MemoryProvider::new().with("/w/res.va", MODEL_VA);
    // set for the file: read as SPICE, which it is not, and warned of as SPICE warns
    let d = build("/w/res.va", &files, &[("/w/res.va", "spice")]);
    assert_eq!((d.dialect.as_str(), formats_of(&d)[0]), ("spice", ("res.va", Some("spice"), Some("text"))));
    assert!(d.diagnostics.iter().any(|x| x.message.contains("not in the supported SPICE subset")), "{:?}", said(&d));
    assert!(!d.diagnostics.iter().any(|x| x.message.contains("is not a netlist")));
    // the dialect of the opened file, too
    let options = BuildOptions { dialect: Some("spice".into()), ..BuildOptions::default() };
    let d = rnl_view::build("/w/res.va", &files, &options).unwrap();
    assert_eq!(formats_of(&d)[0], ("res.va", Some("spice"), Some("spice")));
    // and any file can be set to text: a netlist then is not read either
    let files = MemoryProvider::new().with("/w/top.sp", "* t\nR1 a 0 1k\n");
    let d = build("/w/top.sp", &files, &[("/w/top.sp", "text")]);
    assert_eq!(formats_of(&d)[0], ("top.sp", Some("text"), Some("spice")));
    assert_eq!(said(&d), [("hint", "top.sp", 1, "`top.sp` is read as text, not as a netlist; it is not read")]);
    assert!(d.objects.is_empty());
}

#[test]
fn a_verilog_a_file_that_a_netlist_includes_is_kept_as_text() {
    // SPICE's `.include` and Spectre's `include` follow it, and read nothing of it: the hint is at
    // the include; Spectre's `ahdl_include` does not follow it (host_regions.rs)
    for (top, text) in [
        ("/w/top.sp", "* top\n.include \"res.va\"\nR1 a 0 1k\n.end\n"),
        ("/w/top.scs", "// top\ninclude \"res.va\"\nR1 (a 0) resistor r=1k\n"),
    ] {
        let files = MemoryProvider::new().with(top, text).with("/w/res.va", MODEL_VA);
        let d = build(top, &files, &[]);
        let main = &top[3..];
        let format = if main.ends_with(".sp") { "spice" } else { "spectre" };
        assert_eq!(
            formats_of(&d),
            [(main, Some(format), Some(format)), ("res.va", Some("text"), Some("text"))],
            "{top}"
        );
        assert_eq!(d.files[0].includes.len(), 1);
        let notes: Vec<_> = said(&d).into_iter().filter(|x| x.0 != "hint" || x.3.contains("res.va")).collect();
        assert_eq!(notes, [("hint", main, 2, "`res.va` is not a netlist (Verilog-A); it is not read")], "{top}");
        assert_eq!(
            d.objects.iter().filter(|o| o.kind == "instance").map(|o| o.name.as_str()).collect::<Vec<_>>(),
            ["R1"]
        );
        assert!(d.files[1].records.is_empty());
        // set for it, it is read with that format
        let d = build(top, &files, &[("/w/res.va", "spice")]);
        assert_eq!(formats_of(&d)[1], ("res.va", Some("spice"), Some("text")));
    }
}

#[test]
fn check_of_a_verilog_a_file_prints_the_hint_and_succeeds() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-formats-text");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("res.va"), MODEL_VA).unwrap();
    let va = rnl_cli::path_str(&dir.join("res.va"));
    let (code, out, err) = rnl(&["check", &va]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out,
        "hint res.va:1: `res.va` is not a netlist (Verilog-A); it is not read
0 errors, 0 warnings, 1 hints
"
    );
    // a format set for it reads it as SPICE, with SPICE's warnings
    let (code, out, _) = rnl(&["check", &va, "--format", &format!("{va}=spice")]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("not in the supported SPICE subset"), "{out}");
}
