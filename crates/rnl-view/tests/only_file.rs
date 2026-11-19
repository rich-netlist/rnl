//! A reading of one file of a document (`Options::only_file`) builds the levels the reading of the
//! whole document builds, and reports what it does about them (`rnl_runtime::per_file`): on every
//! netlist of the repository's corpora, and on documents made for what reaches one file from
//! others.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use rnl_runtime::per_file;
use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_runtime::Options;

/// The files of the repository, read from the disk.
struct Disk;

impl Provider for Disk {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

/// Read each file of the document `entry` alone and compare it with the reading of the whole, with
/// `options` and the problems of the levels: no difference, and nothing the whole reading reports
/// that no reading of one file does. Gives the titles of the levels each file builds.
fn each_file(entry: &str, files: &dyn Provider, options: &Options) -> Vec<(String, Vec<String>)> {
    let formats = BTreeMap::new();
    let options = Options { problems: true, ..options.clone() };
    let whole = rnl_view::interpret_with(entry, None, &formats, files, &options).unwrap();
    let mut found: HashSet<String> = HashSet::new();
    let mut built = Vec::new();
    for file in per_file::files(&whole.doc) {
        let alone = Options { only_file: Some(file.clone()), ..options.clone() };
        let part = rnl_view::interpret_with(entry, None, &formats, files, &alone).unwrap();
        let differences = per_file::differences(&whole, &part, &file);
        assert!(differences.is_empty(), "{entry}, {file}:\n{}", differences.join("\n"));
        found.extend(per_file::reported(&part));
        built.push((file, part.model.levels.iter().map(|l| l.title.clone()).collect()));
    }
    let unreported = per_file::unreported(&whole, &found);
    assert!(unreported.is_empty(), "{entry}:\n{}", unreported.join("\n"));
    built
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
fn each_file_of_the_corpora_reads_alone_as_in_the_whole_document() {
    let mut dirs = vec!["tests/conformance/cases-0.4/from-0.2", "tests/conformance/cases-0.4/from-0.3"];
    dirs.extend(["tests/conformance/equivalence-0.4", "skills/rnl/examples"]);
    dirs.extend(["tests/integration/solver-0.4", "tests/integration/solver-0.4/solved"]);
    for dir in dirs {
        for entry in netlists(dir) {
            each_file(&entry, &Disk, &Options::default());
        }
    }
}

#[test]
fn each_file_reads_alone_as_a_solver_reads_it() {
    for entry in netlists("tests/integration/solver-0.4/solved") {
        each_file(&entry, &Disk, &Options::solver());
    }
}

const TOP: &str = "* top
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }
.include \"mid.inc\"
.include \"leaf.inc\"
X1 a b cell
VP1 a 0 1
VP2 b 0 1
* @RNL {
*   %vs = {#VP1 #VP2};
*   # @SELECT-INSTANCE(name=\"R*\", recursive=true) => @SET-STYLE $grey
* }
* @RNL(scope=\"cell\") {
*   #R1 @TEXT(\"R_{1}\", key=\"name\")
* }
.end
";

/// An included file with no circuit of its own, whose rule reaches the files it includes.
const MID: &str = "* mid
* @RNL {
*   # @SELECT-INSTANCE(name=\"C*\", recursive=true) => @SET-STYLE $signal
* }
.include \"cells.inc\"
";

/// A subcircuit that uses a binding of the main file, and a subcircuit of another file.
const CELLS: &str = "* cells
.subckt cell p n
R1 p n 1k
C1 p n 1p
X9 p n leaf
* @RNL {
*   %vs @SET-STYLE $grey;
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

#[test]
fn what_reaches_a_file_from_the_files_above_and_the_files_below_it_is_read_alone_too() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", TOP)
        .with("/w/mid.inc", MID)
        .with("/w/cells.inc", CELLS)
        .with("/w/leaf.inc", LEAF);
    let built = each_file("/w/top.sp", &files, &Options::default());
    let titles: Vec<(&str, Vec<&str>)> =
        built.iter().map(|(f, l)| (f.as_str(), l.iter().map(String::as_str).collect())).collect();
    assert_eq!(
        titles,
        [("top.sp", vec!["top.sp"]), ("mid.inc", vec![]), ("cells.inc", vec!["cell"]), ("leaf.inc", vec!["leaf"])]
    );
    // the rules of the main file and of the included file reach the subcircuit read alone
    let alone = Options { only_file: Some("cells.inc".to_string()), ..Options::default() };
    let r = rnl_view::interpret_with("/w/top.sp", None, &BTreeMap::new(), &files, &alone).unwrap();
    let cell = &r.model.levels[0];
    let style = |name: &str| {
        let item = cell.items.iter().find(|i| i.object_id == format!("#{name}")).unwrap();
        format!("{:?}", item.style)
    };
    assert!(style("R1").contains("#9a9a9a"), "{}", style("R1"));
    assert_ne!(style("C1"), style("R1"));
}

#[test]
fn a_file_included_twice_reads_alone_as_in_the_whole_document() {
    let top = "* top
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }
.include \"twice.inc\"
.include \"twice.inc\"
X1 a 0 r2
.end
";
    let twice = "* twice
.subckt r2 a b
R1 a b 1k
* @RNL { #R1 @PLACE(0, 0) }
.ends
";
    let files = MemoryProvider::new().with("/w/top.sp", top).with("/w/twice.inc", twice);
    each_file("/w/top.sp", &files, &Options::default());
}
