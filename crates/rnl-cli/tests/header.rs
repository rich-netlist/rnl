//! The header record (2.8, 16.1): at the file level, the first record of the whole file, and at
//! most one. One written after a record of the file, also after a record in a subcircuit above
//! it, or inside a subcircuit, or a second one, is an error and is ignored as a whole, VERSION
//! included; the file then reads as one without a header record. A file without a version takes
//! the version of the file that includes it, and the main file the tool's default. A file declares
//! its version once: a second VERSION is an error and ignored (16.1).

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

const HEADER: &str = "* @RNL(type=\"header\") { @VERSION(\"0.4\")}";
const AFTER: &str =
    "header records precede all other records, with the one without language first (2.8); this one is ignored";
const NO_HEADER: &str = "the main file has no header record: it is read in the tool's default version 0.4, and tools add the header when they write the file (16.1)";

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
            "a file has at most one header record without language and one per language (2.8); this one is ignored",
        ),
    ];
    for (text, line, message) in cases {
        let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
        assert_eq!(said(&r, Severity::Error), [("t.sp".to_string(), line, message.to_string())], "{text}");
    }
}

#[test]
fn a_second_version_is_an_error_and_the_first_stands() {
    // a file never has two version values (16.1): the second VERSION of its header record is an
    // error and ignored, so the file reads in the first, 0.4, where T has terminal names (6.12);
    // the first being 0.3, the file is not read at all
    for (first, second) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{first}\"); @VERSION(\"{second}\") }}\n\
             T1 a b c d z0=50 td=1n\n* @RNL {{ #T1:p1 @TEXT(\"first port\") }}\n.end\n"
        );
        let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
        let errors: Vec<String> = said(&r, Severity::Error).into_iter().map(|(_, _, m)| m).collect();
        let second = rnl_runtime::interp::unit::SECOND_VERSION.to_string();
        if first == "0.4" {
            assert_eq!(errors, std::slice::from_ref(&second), "{first}");
            assert_eq!(said(&r, Severity::Error)[0].1, 2, "{first}");
        } else {
            assert_eq!(errors.len(), 1, "{first}: {errors:?}");
            assert!(errors[0].starts_with("unsupported version 0.3"), "{first}: {errors:?}");
        }
    }
}

#[test]
fn an_included_file_without_a_version_takes_the_includers() {
    // the main file declares 0.4 or nothing, which is the tool's default 0.4; the included file
    // declares none, and its subcircuit is read in the main file's version, as is a file that
    // declares 0.4 itself (16.1)
    let a = "* a\n.subckt cell p\nR1 p 0 1k\n.ends\n";
    let b = "* b\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.subckt other p\nR1 p 0 1k\n.ends\n";
    let body = ".include \"a.inc\"\n.include \"b.inc\"\nX1 n cell\nX2 n other\n.end\n";
    for (header, version) in [("@VERSION(\"0.4\")", "0.4"), ("", "0.4")] {
        let main = match header {
            "" => format!("* t\n{body}"),
            _ => format!("* t\n* @RNL(type=\"header\") {{ {header}}}\n{body}"),
        };
        let files = MemoryProvider::new().with("t.sp", &main).with("a.inc", a).with("b.inc", b);
        let r = read(&files, &Options::solver());
        let read_in = |title: &str| r.problems.iter().find(|p| p.title == title).unwrap().version.clone();
        assert_eq!(
            (read_in("t.sp"), read_in("cell"), read_in("other")),
            (version.into(), version.into(), "0.4".into()),
            "{main}"
        );
    }
}

#[test]
fn a_section_of_lib_counts_as_a_file() {
    // each section read is a file of its own, whose header record is the first record in it (2.8):
    // the records of the section tt, above, do not come before the header record of ff
    let main = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.lib 'models.lib' tt\n.lib 'models.lib' ff\n\
                X1 n cella\nX2 n cellb\n.end\n";
    let lib = "* models\n.lib tt\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.subckt cella a\nR1 a 0 1k\n\
               * @RNL {#R1 @PLACE(0, 0)}\n.ends\n.endl tt\n.lib ff\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n\
               .subckt cellb a\nR1 a 0 1k\n.ends\n.endl ff\n";
    let r = read(&MemoryProvider::new().with("t.sp", main).with("models.lib", lib), &Options::default());
    assert!(said(&r, Severity::Error).is_empty(), "{:?}", said(&r, Severity::Error));
    // in a section, a record of a subcircuit above the header record still comes first
    let lib = "* models\n.lib tt\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.subckt cella a\nR1 a 0 1k\n.ends\n\
               .endl tt\n.lib ff\n.subckt cellb a\nR1 a 0 1k\n* @RNL {#R1 @PLACE(0, 0)}\n.ends\n\
               * @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.endl ff\n";
    let r = read(&MemoryProvider::new().with("t.sp", main).with("models.lib", lib), &Options::default());
    assert_eq!(said(&r, Severity::Error), [("models.lib".to_string(), 13, AFTER.to_string())]);
}

#[test]
fn repeated_record_head_fields_are_absent_without_losing_the_body() {
    for (name, values) in [
        ("type", "\"generated\""),
        ("priority", "\"disabled\""),
        ("scope", "\"missing\""),
        ("numbering", "\"0000000000000000\""),
        ("author", "\"tool\""),
        ("hash", "\"stale\""),
        ("modified", "\"2026-10-18T00:00:00Z\""),
    ] {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n\
             * @RNL({name}={values}, {name}={values}, {name}={values}) {{ (1, 2) @TEXT(\"kept\") }}\n.end\n"
        );
        // In particular, two generated types fall back to authored, so the solver reads the body.
        let r = read(&MemoryProvider::new().with("t.sp", &text), &Options { std: false, ..Options::solver() });
        let errors = said(&r, Severity::Error);
        assert_eq!(errors.len(), 1, "{name}: {errors:?}");
        assert!(errors[0].2.contains(&format!("field {name} is written more than once")), "{errors:?}");
        assert_eq!(r.model.levels[0].texts.iter().filter(|t| t.markup == "kept").count(), 1, "{name}");
        let record = r.doc.levels[0].records.last().unwrap();
        assert_eq!(rnl_runtime::writeback::field(record, name), None, "{name}");
        assert_eq!(rnl_runtime::writeback::record_type(record), "authored", "{name}");
        assert_eq!(rnl_runtime::writeback::strip_generated(&r.doc, "t.sp", &text), text, "{name}");
    }
}

#[test]
fn a_repeated_head_field_keeps_other_fields_and_keyword_duplicates_still_fail() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n\
                * @RNL(priority=\"low\", priority=\"high\", author=\"kept\") {\n\
                * (1, 2) @TEXT(\"body\"); (3, 4) @TEXT(\"bad\", key=\"a\", key=\"b\"); (5, 6) @TEXT(\"after\")\n\
                * }\n.end\n";
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options { std: false, ..Options::default() });
    let errors = said(&r, Severity::Error);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors.iter().any(|e| e.2.contains("parameter key of @TEXT is written twice")), "{errors:?}");
    let texts: Vec<_> = r.model.levels[0].texts.iter().map(|t| t.markup.as_str()).collect();
    assert_eq!(texts, ["body", "after"]);
    let record = r.doc.levels[0].records.last().unwrap();
    assert_eq!(rnl_runtime::writeback::field(record, "author").as_deref(), Some("kept"));
    assert_eq!(rnl_runtime::writeback::field(record, "priority"), None);
}
