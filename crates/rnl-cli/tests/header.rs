//! The header record (2.8, 16.1): at the file level, the first record of the whole file, and at
//! most one. One written after a record of the file, also after a record in a subcircuit above
//! it, or inside a subcircuit, or a second one, is an error and is ignored as a whole, VERSION
//! included; the file then reads as one without a header record. A file without a version takes
//! the version of the file that includes it, and the main file the tool's default.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_runtime::{Interpretation, Options};

fn read(files: &dyn Provider, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, files, options).unwrap()
}

/// The diagnostics of one severity, each by its file, line and message.
fn said(r: &Interpretation, severity: Severity) -> Vec<(String, u32, String)> {
    let found = r.diagnostics().into_iter().filter(|d| d.severity == severity);
    found
        .map(|d| {
            (
                d.loc.as_ref().map_or(String::new(), |l| l.file.clone()),
                d.loc.as_ref().map_or(0, |l| l.line),
                d.message.clone(),
            )
        })
        .collect()
}

const HEADER: &str = "* @RNL(type=\"header\") { @VERSION(\"0.2\")}";
const AFTER: &str =
    "a header record is the first record of a file, and a record comes before this one (2.8); this one is ignored";
const NO_HEADER: &str = "the main file has no header record: it is read in the tool's default version 0.3, and tools add the header when they write the file (16.1)";

#[test]
fn a_record_in_a_subcircuit_above_it_comes_first() {
    // the record of the subcircuit is the first record of the file, so the header record that
    // follows it at the file level is not the first: an error, and the file has no header
    let text =
        format!("* t\n.subckt cell a\nR1 a 0 1k\n* @RNL {{#R1 @PLACE(0, 0)}}\n.ends\n{HEADER}\nX1 n cell\n.end\n");
    let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
    assert_eq!(said(&r, Severity::Error), [("t.sp".to_string(), 6, AFTER.to_string())]);
    assert!(said(&r, Severity::Hint).contains(&("t.sp".to_string(), 2, NO_HEADER.to_string())));
    // on line 2, above the subcircuit, it is the header record
    let text =
        format!("* t\n{HEADER}\n.subckt cell a\nR1 a 0 1k\n* @RNL {{#R1 @PLACE(0, 0)}}\n.ends\nX1 n cell\n.end\n");
    let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
    assert!(said(&r, Severity::Error).is_empty(), "{:?}", said(&r, Severity::Error));
    assert!(!said(&r, Severity::Hint).iter().any(|d| d.2 == NO_HEADER));
}

#[test]
fn only_the_first_record_of_the_file_level_and_only_one() {
    let cases = [
        // after a record of the file level
        (format!("* t\nR1 a 0 1k\n* @RNL {{#R1 @PLACE(0, 0)}}\n{HEADER}\n.end\n"), 4, AFTER),
        // inside a subcircuit
        (
            format!("* t\n.subckt cell a\n{HEADER}\nR1 a 0 1k\n.ends\n.end\n"),
            3,
            "a header record is written at the file level, not inside a subcircuit (2.8); this one is ignored",
        ),
        // a second one
        (
            format!("* t\n{HEADER}\n{HEADER}\nR1 a 0 1k\n.end\n"),
            3,
            "a file has at most one header record (2.8); this one is ignored",
        ),
    ];
    for (text, line, message) in cases {
        let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
        assert_eq!(said(&r, Severity::Error), [("t.sp".to_string(), line, message.to_string())], "{text}");
    }
}

#[test]
fn an_included_file_without_a_version_takes_the_includers() {
    // the main file declares 0.3, 0.2 or nothing, which is the tool's default 0.3; the included
    // file declares none, and its subcircuit is read in the main file's version, while a file that
    // declares 0.2 stays 0.2 (16.1)
    let a = "* a\n.subckt cell p\nR1 p 0 1k\n.ends\n";
    let b = "* b\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt other p\nR1 p 0 1k\n.ends\n";
    let body = ".include \"a.inc\"\n.include \"b.inc\"\nX1 n cell\nX2 n other\n.end\n";
    for (header, version) in [("@VERSION(\"0.3\")", "0.3"), ("@VERSION(\"0.2\")", "0.2"), ("", "0.3")] {
        let main = match header {
            "" => format!("* t\n{body}"),
            _ => format!("* t\n* @RNL(type=\"header\") {{ {header}}}\n{body}"),
        };
        let files = MemoryProvider::new().with("t.sp", &main).with("a.inc", a).with("b.inc", b);
        let r = read(&files, &Options::solver());
        let read_in = |title: &str| r.problems.iter().find(|p| p.title == title).unwrap().version.clone();
        assert_eq!(
            (read_in("t.sp"), read_in("cell"), read_in("other")),
            (version.into(), version.into(), "0.2".into()),
            "{main}"
        );
    }
}

#[test]
fn a_section_of_lib_counts_as_a_file() {
    // each section read is a file of its own, whose header record is the first record in it (2.8):
    // the records of the section tt, above, do not come before the header record of ff
    let main = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.lib 'models.lib' tt\n.lib 'models.lib' ff\n\
                X1 n cella\nX2 n cellb\n.end\n";
    let lib = "* models\n.lib tt\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt cella a\nR1 a 0 1k\n\
               * @RNL {#R1 @PLACE(0, 0)}\n.ends\n.endl tt\n.lib ff\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n\
               .subckt cellb a\nR1 a 0 1k\n.ends\n.endl ff\n";
    let r = read(&MemoryProvider::new().with("t.sp", main).with("models.lib", lib), &Options::default());
    assert!(said(&r, Severity::Error).is_empty(), "{:?}", said(&r, Severity::Error));
    // in a section, a record of a subcircuit above the header record still comes first
    let lib = "* models\n.lib tt\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt cella a\nR1 a 0 1k\n.ends\n\
               .endl tt\n.lib ff\n.subckt cellb a\nR1 a 0 1k\n* @RNL {#R1 @PLACE(0, 0)}\n.ends\n\
               * @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.endl ff\n";
    let r = read(&MemoryProvider::new().with("t.sp", main).with("models.lib", lib), &Options::default());
    assert_eq!(said(&r, Severity::Error), [("models.lib".to_string(), 13, AFTER.to_string())]);
}
