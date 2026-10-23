//! File-local language versions, inherited names/packages and writeback boundaries (16.1).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_cli::solve::{solve, SolveOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(files: &MemoryProvider, std: bool) -> Interpretation {
    interpret_file(Path::new("/w/top.sp"), None, files, &Options { std, ..Options::solver() }).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

fn texts(r: &Interpretation, title: &str) -> usize {
    r.model.levels.iter().find(|l| l.title == title).unwrap().texts.len()
}

#[test]
fn an_explicit_supported_child_restarts_version_inheritance() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", "* top\n* @RNL(type=\"header\") { @USE-PACKAGE(name=\"missing\", path=\"missing.rnl\"); @VERSION(\"9.9\") }\n.include good.inc\n.include inherited.inc\n* @RNL { (0,0) @TEXT(\"ignored parent\") }\n.end\n")
        .with("/w/good.inc", "* included file\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt good p n\n* @RNL { (0,0) @TEXT(\"good\") }\n.ends\n.include next.inc\n")
        .with("/w/next.inc", ".subckt next p n\n* @RNL { (0,0) @TEXT(\"inherits 0.3\") }\n.ends\n")
        .with("/w/inherited.inc", ".subckt bad p n\n* @RNL { (0,0) @TEXT(\"inherits 9.9\") }\n.ends\n.include old.inc\n")
        .with("/w/old.inc", "* included file\n* @RNL(type=\"header\") { @VERSION(\"0.2\") }\n.subckt old p n\n* @RNL { (0,0) @TEXT(\"old\") }\n.ends\n");
    let r = read(&files, true);
    let errs = errors(&r);
    assert_eq!(errs.len(), 2, "{errs:?}");
    assert!(errs.iter().all(|e| e.contains("unsupported version 9.9")), "{errs:?}");
    assert!(!r.sources.iter().any(|(p, _)| p.contains("missing")));
    for title in ["good", "next", "old"] {
        assert_eq!(texts(&r, title), 1, "{title}");
        let lv = r.model.levels.iter().find(|l| l.title == title).unwrap();
        let std = lv.packages.iter().find(|p| p.name == "rnl-std").unwrap();
        assert_eq!(std.version, if title == "old" { "0.2" } else { "0.3" });
    }
    for title in ["top.sp", "bad"] {
        assert_eq!(texts(&r, title), 0, "{title}");
    }
    for (lv, unsupported) in r.doc.levels.iter().zip(&r.unsupported) {
        let bad = lv.file.ends_with("top.sp") || lv.file.ends_with("inherited.inc");
        assert_eq!(unsupported.is_some(), bad, "{}: {unsupported:?}", lv.name);
    }
}

#[test]
fn an_unread_middle_file_contributes_nothing_but_does_not_hide_supported_ancestors() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", "* top\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-style\", version=\"0.3\") }\n* @RNL { $GOOD = @DEFINE-SYMBOL { (0,0) @TEXT(\"inherited\"); (0,0) @PIN(terminal=1) } }\n.include middle.inc\n.end\n")
        .with("/w/middle.inc", "* included file\n* @RNL(type=\"header\") { @USE-PACKAGE(name=\"missing\", path=\"missing.rnl\"); @VERSION(\"9.9\") }\n* @RNL { $BAD = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; $GOOD = @DEFINE-SYMBOL { (0,0) @TEXT(\"must not shadow\") } }\n.include child.inc\n")
        .with("/w/child.inc", "* included file\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt child p n\nR1 p n 1k\nR2 p n 1k\n* @RNL { #R1 @SET-SYMBOL $GOOD => @PLACE(0,0); #R2 @SET-SYMBOL $BAD }\n.ends\n");
    let r = read(&files, false);
    let errs = errors(&r);
    assert_eq!(errs.len(), 2, "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("unsupported version 9.9")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("BAD")), "{errs:?}");
    assert!(!errs.iter().any(|e| e.contains("missing") || e.contains("GOOD")), "{errs:?}");
    let lv = r.model.levels.iter().find(|l| l.title == "child").unwrap();
    assert!(lv.packages.iter().any(|p| p.name == "rnl-std-style" && p.version == "0.3"));
    assert_eq!(lv.items.iter().find(|i| i.host == [0]).unwrap().texts.len(), 1);
}

#[test]
fn solving_a_supported_child_does_not_write_into_its_unread_parent() {
    let parent = "* top\n* @RNL(type=\"header\") { @VERSION(\"9.9\") }\n.include child.inc\nRtop a 0 1k\n.end\n";
    let files = MemoryProvider::new()
        .with("/w/top.sp", parent)
        .with("/w/child.inc", "* included file\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt child p n\nR1 p n 1k\n* @RNL { #R1 @PLACE(0,0) }\n.ends\n");
    let run = solve(Path::new("/w/top.sp"), &files, &SolveOptions { std: false, ..SolveOptions::default() }).unwrap();
    assert_eq!(run.levels.iter().map(|l| l.problem.title.as_str()).collect::<Vec<_>>(), ["child"]);
    assert_eq!(run.not_solved.len(), 1);
    assert_eq!(run.not_solved[0].title, "top.sp");
    assert_eq!(run.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["/w/child.inc"]);
    assert!(run.files[0].text.contains("type=\"generated\""));
    assert_eq!(run.read.iter().filter(|p| p.ends_with("top.sp")).count(), 1);
}

#[test]
fn selected_library_sections_each_establish_their_own_version_boundary() {
    let files = MemoryProvider::new()
        .with("/w/top.sp", "* top\n* @RNL(type=\"header\") { @VERSION(\"9.9\") }\n.lib parts.lib good\n.lib parts.lib inherited\n.end\n")
        .with("/w/parts.lib", ".lib good\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt good p n\n* @RNL { (0,0) @TEXT(\"good\") }\n.ends\n.endl\n.lib inherited\n.subckt bad p n\n* @RNL { (0,0) @TEXT(\"bad\") }\n.ends\n.endl\n");
    let r = read(&files, false);
    assert_eq!(texts(&r, "good"), 1);
    assert_eq!(texts(&r, "bad"), 0);
    assert_eq!(errors(&r).len(), 2, "{:?}", errors(&r));
}
