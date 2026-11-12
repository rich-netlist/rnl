//! Auto Include success is informational; its source reading selects the presentation.
use rnl_runtime::{
    diag::{codes, Severity},
    source::MemoryProvider,
    Options,
};
use rnl_view::{LibraryIndex, LibraryProvider};

fn header(version: &str) -> String {
    format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n")
}

fn run(files: &MemoryProvider, paths: &[&str]) -> rnl_runtime::Interpretation {
    let mut index = LibraryIndex::default();
    assert!(index.update(&paths.iter().map(|p| p.to_string()).collect::<Vec<_>>(), files, false).is_empty());
    let provider = LibraryProvider { files, index: &index, enabled: true };
    rnl_view::interpret("/top.sp", None, &provider, &Options { std: false, ..Default::default() }).unwrap()
}

fn success(result: &rnl_runtime::Interpretation) -> Vec<&rnl_runtime::diag::Diag> {
    result.diagnostics().into_iter().filter(|d| d.code == codes::VIEW_AUTOINCLUDE_ADDED).collect()
}

#[test]
fn modern_success_is_a_hint_at_the_requesting_instance() {
    let files = MemoryProvider::new()
        .with("/top.sp", &(header("0.4") + "X1 a b cell\n"))
        .with("/lib.inc", ".subckt cell p n\n.ends\n");
    let result = run(&files, &["/lib.inc"]);
    let diagnostics = success(&result);
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Hint);
    assert_eq!(diagnostic.message, "subcircuit cell was added from /lib.inc by Auto Include");
    let loc = diagnostic.loc.as_ref().unwrap();
    assert_eq!((loc.file.as_str(), loc.line), ("top.sp", 3));
    assert!(result.doc.subckt("cell").is_some());
}

#[test]
fn released_success_retains_the_exact_warning() {
    for version in ["0.2", "0.3"] {
        let files = MemoryProvider::new()
            .with("/top.sp", &(header(version) + "X1 a b cell\n"))
            .with("/lib.inc", ".subckt cell p n\n.ends\n");
        let result = run(&files, &["/lib.inc"]);
        let diagnostics = success(&result);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(
            diagnostics[0].message,
            "Subcircuit cell was not included; found in /lib.inc and added by Auto Include"
        );
    }
}

#[test]
fn explicit_child_version_wins_over_the_main_version() {
    for (root, child, expected) in [("0.4", "0.3", Severity::Warning), ("0.3", "0.4", Severity::Hint)] {
        let files = MemoryProvider::new()
            .with("/top.sp", &(header(root) + ".include child.inc\n"))
            .with("/child.inc", &(header(child) + "X1 a b cell\n"))
            .with("/lib.inc", ".subckt cell p n\n.ends\n");
        let result = run(&files, &["/lib.inc"]);
        let diagnostics = success(&result);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, expected);
        assert_eq!(diagnostics[0].loc.as_ref().unwrap().file, "child.inc");
    }
}

#[test]
fn recursive_additions_use_the_version_of_each_requester() {
    let files = MemoryProvider::new()
        .with("/top.sp", &(header("0.4") + "X1 a b cell\n"))
        .with("/cell.inc", &(header("0.3") + ".subckt cell p n\nX2 p n leaf\n.ends\n"))
        .with("/leaf.inc", ".subckt leaf p n\n.ends\n");
    let result = run(&files, &["/cell.inc", "/leaf.inc"]);
    let diagnostics = success(&result);
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|d| d.severity == Severity::Hint && d.loc.as_ref().unwrap().file == "top.sp"));
    assert!(diagnostics.iter().any(|d| d.severity == Severity::Warning && d.loc.as_ref().unwrap().file == "cell.inc"));
}

#[test]
fn ambiguous_libraries_still_warn_and_are_not_loaded() {
    let files = MemoryProvider::new()
        .with("/top.sp", &(header("0.4") + "X1 a b cell\n"))
        .with("/a.inc", ".subckt cell p n\n.ends\n")
        .with("/b.inc", ".subckt cell p n\n.ends\n");
    let result = run(&files, &["/a.inc", "/b.inc"]);
    assert!(success(&result).is_empty());
    assert!(result.doc.subckt("cell").is_none());
    assert!(result
        .diagnostics()
        .iter()
        .any(|d| d.code == codes::VIEW_AUTOINCLUDE_AMBIGUOUS && d.severity == Severity::Warning));
}

#[test]
fn conflicts_still_warn_and_do_not_replace_existing_definitions() {
    let files = MemoryProvider::new()
        .with("/top.sp", &(header("0.4") + ".subckt existing a b\n.ends\nX1 a b cell\n"))
        .with("/lib.inc", ".subckt cell p n\n.ends\n.subckt existing a b\n.ends\n");
    let result = run(&files, &["/lib.inc"]);
    assert!(success(&result).is_empty());
    assert!(result.doc.subckt("cell").is_none());
    assert!(result
        .diagnostics()
        .iter()
        .any(|d| d.code == codes::VIEW_AUTOINCLUDE_CONFLICT && d.severity == Severity::Warning));
}

#[test]
fn versionless_includes_inherit_the_requesting_version() {
    let files = MemoryProvider::new()
        .with("/top.sp", &(header("0.4") + ".include child.inc\n"))
        .with("/child.inc", "* title\nX1 a b cell\n")
        .with("/lib.inc", ".subckt cell p n\n.ends\n");
    let result = run(&files, &["/lib.inc"]);
    let diagnostics = success(&result);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Hint);
}

#[test]
fn selected_sections_keep_their_individual_versions() {
    let files = MemoryProvider::new()
        .with("/top.sp", &(header("0.3") + ".lib sections.inc old\n.lib sections.inc modern\n"))
        .with(
            "/sections.inc",
            &format!(
                "* title\n.lib old\n{}X1 a b old_cell\n.endl old\n.lib modern\n{}X2 c d modern_cell\n.endl modern\n",
                header("0.3"),
                header("0.4")
            ),
        )
        .with("/lib.inc", ".subckt old_cell p n\n.ends\n.subckt modern_cell p n\n.ends\n");
    let result = run(&files, &["/lib.inc"]);
    let diagnostics = success(&result);
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|d| d.message.contains("old_cell") && d.severity == Severity::Warning));
    assert!(diagnostics.iter().any(|d| d.message.contains("modern_cell") && d.severity == Severity::Hint));
    assert!(diagnostics.iter().all(|d| d.loc.as_ref().unwrap().file == "sections.inc"));
}
