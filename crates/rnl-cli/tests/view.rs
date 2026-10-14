//! The Document over the disk, as the desktop viewer builds it: the path made absolute, as the
//! system writes it, and the files read by `FsProvider`.

use std::path::{Path, PathBuf};

use rnl_cli::{path_str, FsProvider};
use rnl_view::{build, BuildOptions, Document};

/// What the desktop viewer's `open_document` does with a path.
fn open(path: &Path) -> Result<Document, String> {
    let path = std::path::absolute(path).unwrap();
    build(&path.to_string_lossy(), &FsProvider::new(&[]), &BuildOptions::default())
}

/// The corpus files, named relative to the crate, where the tests run.
fn corpus() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in ["cases", "equivalence"] {
        let dir = Path::new("../../tests/conformance").join(dir);
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension().and_then(|e| e.to_str()).is_some_and(|e| ["sp", "scs", "inc", "rnl"].contains(&e))
            })
            .collect();
        files.sort();
        out.extend(files);
    }
    out
}

#[test]
fn an_absolute_path_gives_the_document_of_a_relative_one() {
    let files = corpus();
    assert!(files.len() > 20);
    for f in files {
        let absolute = open(&f).unwrap();
        let relative = build(&path_str(&f), &FsProvider::new(&[]), &BuildOptions::default()).unwrap();
        assert_eq!(absolute.path, std::path::absolute(&f).unwrap().to_string_lossy());
        assert_eq!(absolute.name, f.file_name().unwrap().to_string_lossy());
        assert!(absolute.missing.is_empty(), "{f:?}: {:?}", absolute.missing);
        // everything but the entry itself is named relative to its directory, or is bundled
        let (mut a, mut r) = (serde_json::to_value(absolute).unwrap(), serde_json::to_value(relative).unwrap());
        a["path"] = serde_json::Value::Null;
        r["path"] = serde_json::Value::Null;
        assert!(a == r, "{f:?}");
    }
}

#[test]
fn the_files_are_read_as_the_disk_holds_them() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("view-disk");
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    let top = "* top\n.include \"lib/parts.inc\"\n.include \"lib/none.inc\"\nX1 a b part\n.end\n";
    // UTF-8 with a byte order mark, and an include in Latin-1
    std::fs::write(dir.join("top.sp"), [&[0xef, 0xbb, 0xbf], top.as_bytes()].concat()).unwrap();
    std::fs::write(dir.join("lib/parts.inc"), b"* caf\xe9\n.subckt part p q\nR1 p q 1k\n.ends\n").unwrap();

    let d = open(&dir.join("top.sp")).unwrap();
    assert_eq!(d.name, "top.sp");
    let names: Vec<(&str, &str)> = d.files.iter().map(|f| (f.path.as_str(), f.name.as_str())).collect();
    assert_eq!(names, [("top.sp", "top.sp"), ("lib/parts.inc", "lib/parts.inc")]);
    assert_eq!(d.files[0].text, top);
    assert_eq!(d.files[1].text, format!("* caf{}\n.subckt part p q\nR1 p q 1k\n.ends\n", char::from(0xe9)));
    let r1 = d.objects.iter().find(|o| o.name == "R1").unwrap();
    assert_eq!(r1.lines, Some([1, 3, 3]));
    // the include that is not there: diagnosed in the opened file, and missing by its full path
    let none = d.diagnostics.iter().find(|x| x.message.contains("lib/none.inc")).unwrap();
    assert_eq!((none.file_index, none.line), (Some(0), 3));
    assert_eq!(d.missing, [path_str(&std::path::absolute(dir.join("lib").join("none.inc")).unwrap())]);
}
