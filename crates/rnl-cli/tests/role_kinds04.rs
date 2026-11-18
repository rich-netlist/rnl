//! Every item in a draft role's kind list is a kind name (9.4).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(files: MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn source(version: &str, body: &str) -> String {
    format!("* kinds\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\n* @RNL {{ {body} }}\n")
}
fn body(kinds: &str) -> String {
    format!("$D = @DEFINE(roles=[[\"member\", {kinds}]]) {{}}; %u = #R1 @AS $D; (3,0) @TEXT(\"after\")")
}

#[test]
fn nonstring_kind_entries_fail_the_definition_and_keep_other_members() {
    for kinds in ["[\"instance\", 7]", "[false, \"instance\"]", "[[\"net\"], \"instance\"]"] {
        let r = read(MemoryProvider::new().with("main.sp", &source("0.4", &body(kinds))));
        let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::DEFINE_ROLE_MALFORMED).collect();
        assert_eq!(errors.len(), 1, "{:?}", r.diagnostics());
        assert!(errors[0].message.contains("member"));
        assert!(errors[0].message.contains("kind list item"));
        assert!(r.problems.iter().all(|p| p.usages.iter().all(|u| u.definition != "$D")));
        assert!(r.model.levels.iter().flat_map(|l| &l.texts).any(|t| t.markup == "after"));
    }
}

#[test]
fn valid_kind_alternatives_still_accept_either_kind() {
    let body = "$D = @DEFINE(roles=[[\"member\", [\"instance\", \"net\"]]]) {}; #R1 @AS $D; &a @AS $D";
    let r = read(MemoryProvider::new().with("main.sp", &source("0.4", body)));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert_eq!(r.problems.iter().flat_map(|p| &p.usages).filter(|u| u.definition == "$D").count(), 2);
}

#[test]
fn an_included_definition_reports_at_its_source() {
    let top = format!("{}\n.include child.sp\nX1 a child\n", source("0.4", ""));
    let included = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a\nR2 a 0 2k\n* @RNL { $D = @DEFINE(roles=[[\"member\", [\"instance\", 7]]]) {}; #R2 @AS $D }\n.ends child\n";
    let r = read(MemoryProvider::new().with("main.sp", &top).with("child.sp", included));
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::DEFINE_ROLE_MALFORMED).collect();
    assert!(!errors.is_empty(), "{:?}", r.diagnostics());
    assert!(errors[0].loc.as_ref().unwrap().file.ends_with("child.sp"));
}

#[test]
fn a_bad_package_definition_does_not_hide_an_independent_one() {
    let top = "* kinds\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\", path=\"p.rnl\") }\nR1 a 0 1k\n* @RNL { #R1 @AS $D; #R1 @AS $G }\n";
    let package = "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\", version=\"1.0\") }\n@RNL { $D = @DEFINE(roles=[[\"member\", [\"instance\", false]]]) {}; $G = @DEFINE(roles=[[\"member\", \"instance\"]]) {} }\n";
    let r = read(MemoryProvider::new().with("main.sp", top).with("p.rnl", package));
    assert!(r.diagnostics().iter().any(|d| d.code == codes::DEFINE_ROLE_MALFORMED));
    assert_eq!(r.problems.iter().flat_map(|p| &p.usages).filter(|u| u.definition == "$G").count(), 1);
}
