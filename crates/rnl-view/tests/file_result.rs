//! The reading of one file of a document for a viewer (`rnl_view::build_file`): the levels of the
//! whole document's Document that are the file's, where the subcircuits they instantiate are, what
//! the reading depends on, and the symbols of the subcircuits of a file (`rnl_view::symbols`).

use std::path::Path;

use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_view::{BuildOptions, Document, FileResult};

/// The files of the repository, read from the disk.
struct Disk;

impl Provider for Disk {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

const TOP: &str = "* top
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }
.include \"mid.inc\"
.include \"leaf.inc\"
.include \"other.inc\"
X1 a b cell
VP1 a 0 1
VP2 b 0 1
* @RNL {
*   # @SELECT-INSTANCE(name=\"R*\", recursive=true) => @SET-STYLE $grey
* }
.end
";

/// An included file with no circuit of its own.
const MID: &str = "* mid
* @RNL {
*   # @SELECT-INSTANCE(name=\"C*\", recursive=true) => @SET-STYLE $signal
* }
.include \"cells.inc\"
";

const CELLS: &str = "* cells
.subckt cell p n
R1 p n 1k
C1 p n 1p
X9 p n leaf
X8 p n leaf
* @RNL {
*   #R1 @PLACE(0, 0);
*   #C1 @PLACE(4, 0)
* }
.ends
";

/// A subcircuit with a block symbol, which draws its instances in the levels above.
const LEAF: &str = "* leaf
.subckt leaf a b
R1 a b 1k
* @RNL {
*   %leaf_block = @DEFINE-SYMBOL {
*     (-2, 0) @PIN(terminal=\"a\", direction=\"left\"); (2, 0) @PIN(terminal=\"b\", direction=\"right\");
*     @LINE {(-2, 0) (2, 0)}
*   };
*   # @SET-SYMBOL %leaf_block
* }
.ends
";

/// A file none of the others depends on.
const OTHER: &str = "* other
.subckt other a b
R1 a b 1k
.ends
";

fn workspace(other: &str) -> MemoryProvider {
    MemoryProvider::new()
        .with("/w/top.sp", TOP)
        .with("/w/mid.inc", MID)
        .with("/w/cells.inc", CELLS)
        .with("/w/leaf.inc", LEAF)
        .with("/w/other.inc", other)
}

fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

/// The levels of `whole` that are those of `result`'s file, and `result`'s, each with what does
/// not depend on how the objects are numbered: its host, title, kind, ports, lines and drawing.
fn same_levels(whole: &Document, result: &FileResult) {
    let file_index = whole.files.iter().position(|f| f.path == result.file).unwrap();
    let mine: Vec<&rnl_view::Level> = whole
        .levels
        .iter()
        .filter(|l| match l.lines {
            Some([f, _, _]) => f as usize == file_index,
            None => file_index == 0,
        })
        .collect();
    assert_eq!(mine.len(), result.document.levels.len(), "{}", result.file);
    for (w, p) in mine.iter().zip(&result.document.levels) {
        assert_eq!((w.host, &w.title, w.kind, &w.ports, w.lines), (p.host, &p.title, p.kind, &p.ports, p.lines));
        assert_eq!(w.svg, p.svg, "{} {}", result.file, w.title);
        assert_eq!(w.children.len(), p.children.len());
    }
}

#[test]
fn a_file_read_alone_gives_its_levels_where_its_children_are_and_what_it_depends_on() {
    let files = workspace(OTHER);
    let options = BuildOptions::default();
    let whole = rnl_view::build("/w/top.sp", &files, &options).unwrap();
    for f in &whole.files {
        let result = rnl_view::build_file("/w/top.sp", &f.path, &files, &options).unwrap();
        same_levels(&whole, &result);
        // the files are the outline's, the text only the file's
        let paths: Vec<&str> = result.document.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["top.sp", "mid.inc", "cells.inc", "leaf.inc", "other.inc"]);
        for g in &result.document.files {
            assert_eq!(g.text.is_empty(), g.path != f.path, "{}", g.path);
        }
        // read twice, the same bytes
        let again = rnl_view::build_file("/w/top.sp", &f.path, &files, &options).unwrap();
        assert_eq!(json(&result), json(&again));
    }

    let cells = rnl_view::build_file("/w/top.sp", "cells.inc", &files, &options).unwrap();
    let v = json(&cells);
    assert_eq!(v["file"], "cells.inc");
    assert_eq!(v["levels"][0]["title"], "cell");
    let leaf = whole.levels.iter().find(|l| l.title == "leaf").unwrap().host;
    assert_eq!(v["levels"][0]["children"][0]["level"], serde_json::Value::Null);
    assert_eq!(v["levels"][0]["children"][0]["host"], leaf);
    assert_eq!(
        v["children"],
        serde_json::json!([{ "master": "leaf", "host": leaf, "file": "leaf.inc", "title": "leaf" }])
    );
    let roles: Vec<(&str, &str)> = v["inputs"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["role"] != "package")
        .map(|i| (i["role"].as_str().unwrap(), i["path"].as_str().unwrap()))
        .collect();
    assert_eq!(roles, [("body", "cells.inc"), ("context", "top.sp"), ("context", "mid.inc"), ("symbol", "leaf.inc")]);
    let packages: Vec<&serde_json::Value> =
        v["inputs"]["files"].as_array().unwrap().iter().filter(|i| i["role"] == "package").collect();
    assert!(!packages.is_empty() && packages.iter().all(|p| p["bundled"] == true), "{packages:?}");
    assert_eq!(
        v["inputs"]["outline"]["calls"],
        serde_json::json!([{ "master": "leaf", "file": "leaf.inc", "title": "leaf" }])
    );
    assert_eq!(v["inputs"]["format"], 1);
    // the hashes are the outline's
    let outline = rnl_view::outline("/w/top.sp", &files, &options).unwrap();
    for i in v["inputs"]["files"].as_array().unwrap().iter().filter(|i| i["role"] != "package") {
        let f = outline.files.iter().find(|f| f.path == i["path"]).unwrap();
        assert_eq!(i["hash"], f.text_hash.as_str());
        if i["role"] == "context" {
            assert_eq!(i["narrow"], f.records_hash.as_str());
        }
    }
    // the symbol hash of the file of a subcircuit instantiated is the one its symbols give
    let symbols = rnl_view::symbols("/w/top.sp", "leaf.inc", &files, &options).unwrap();
    let narrow =
        v["inputs"]["files"].as_array().unwrap().iter().find(|i| i["path"] == "leaf.inc").unwrap()["narrow"].clone();
    assert_eq!(narrow, symbols.hash.as_str());
}

#[test]
fn a_change_in_a_file_a_reading_does_not_depend_on_leaves_it_as_it_was() {
    let options = BuildOptions::default();
    let before = rnl_view::build_file("/w/top.sp", "cells.inc", &workspace(OTHER), &options).unwrap();
    // another subcircuit and an error of reading in a file nothing above or below cells.inc
    let other = format!("{OTHER}.subckt more a b\nR2 a b 2k\n.ends\n.include \"nowhere.inc\"\n");
    let after = rnl_view::build_file("/w/top.sp", "cells.inc", &workspace(&other), &options).unwrap();
    // but for what the whole document misses, which reading a file reads too
    assert_eq!(after.document.missing, ["/w/nowhere.inc"]);
    let (mut a, mut b) = (json(&before), json(&after));
    a["missing"] = serde_json::Value::Null;
    b["missing"] = serde_json::Value::Null;
    assert_eq!(a, b);
    // and a change in the subcircuit it instantiates, its symbol kept, changes only that input
    let files = workspace(OTHER).with("/w/leaf.inc", &LEAF.replace("R1 a b 1k", "R1 a b 2k"));
    let changed = rnl_view::build_file("/w/top.sp", "cells.inc", &files, &options).unwrap();
    let input = |r: &FileResult| r.inputs.files.iter().find(|i| i.path == "leaf.inc").unwrap().clone();
    assert_ne!(input(&before).hash, input(&changed).hash);
    assert_eq!(input(&before).narrow, input(&changed).narrow);
    let (mut a, mut b) = (json(&before), json(&changed));
    a["inputs"]["files"] = serde_json::Value::Null;
    b["inputs"]["files"] = serde_json::Value::Null;
    assert_eq!(a, b);
}

#[test]
fn the_symbols_of_a_file_are_those_its_subcircuits_give_themselves() {
    let files = workspace(OTHER);
    let options = BuildOptions::default();
    let leaf = rnl_view::symbols("/w/top.sp", "leaf.inc", &files, &options).unwrap();
    assert_eq!(leaf.subckts.len(), 1);
    let s = &leaf.subckts[0];
    assert_eq!((s.title.as_str(), &s.ports), ("leaf", &vec!["a".to_string(), "b".to_string()]));
    let symbol = s.symbol.as_ref().unwrap();
    assert_eq!(symbol.pins.len(), 2);
    // the symbol the levels above draw its instances with
    let whole = rnl_view::build("/w/top.sp", &files, &options).unwrap();
    let cell = whole.levels.iter().find(|l| l.title == "cell").unwrap();
    let used = &whole.symbols[cell.children[0].symbol.unwrap()];
    assert_eq!((&used.name, &used.svg), (&symbol.name, &symbol.svg));
    // a subcircuit that gives itself none
    let cells = rnl_view::symbols("/w/top.sp", "cells.inc", &files, &options).unwrap();
    assert!(cells.subckts[0].symbol.is_none());
    assert_eq!(cells.subckts[0].ports, ["p", "n"]);
    // a file of no subcircuit, and one not in the document
    assert!(rnl_view::symbols("/w/top.sp", "mid.inc", &files, &options).unwrap().subckts.is_empty());
    assert!(rnl_view::symbols("/w/top.sp", "none.inc", &files, &options).is_err());
    assert!(rnl_view::build_file("/w/top.sp", "none.inc", &files, &options).is_err());
}

/// The netlists of a directory of the repository.
fn netlists(dir: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(dir);
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sp" || x == "scs"))
        .map(|p| rnl_runtime::source::normalize(&p.to_string_lossy().replace('\\', "/")))
        .collect();
    out.sort();
    out
}

#[test]
fn each_file_of_the_corpora_draws_its_levels_as_the_whole_document_does() {
    let options = BuildOptions::default();
    let mut dirs = vec!["tests/conformance/cases-0.4/from-0.2", "tests/conformance/cases-0.4/from-0.3"];
    dirs.extend(["tests/conformance/equivalence-0.4", "skills/rnl/examples", "tests/integration/solver-0.4"]);
    for dir in dirs {
        for entry in netlists(dir) {
            let whole = rnl_view::build(&entry, &Disk, &options).unwrap();
            for f in &whole.files {
                let result = rnl_view::build_file(&entry, &f.path, &Disk, &options).unwrap();
                same_levels(&whole, &result);
            }
        }
    }
}
