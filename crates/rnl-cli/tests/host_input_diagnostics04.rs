//! Host input issues are reported by the version of the reading that encountered them.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn source(version: &str, mode: &str, body: &str) -> String {
    let comment = if mode == "spice" { "*" } else { "//" };
    let switch = if mode == "spectre-spice" { "simulator lang=spice\n" } else { "" };
    format!(
        "{comment} input diagnostics\n{comment} @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n{switch}{body}"
    )
}
fn files(path: &str, provider: MemoryProvider) -> Interpretation {
    interpret_file(Path::new(path), None, &provider, &Options { std: false, ..Default::default() }).unwrap()
}
fn read(version: &str, mode: &str, body: &str) -> Interpretation {
    let path = if mode == "spice" { "main.sp" } else { "main.scs" };
    files(path, MemoryProvider::new().with(path, &source(version, mode, body)))
}
fn count(r: &Interpretation, code: Code) -> usize {
    r.diagnostics().iter().filter(|d| d.code == code).count()
}
fn diagnosed(r: &Interpretation, code: Code, severity: Severity, line: u32) {
    let all: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == code).collect();
    assert_eq!(all.len(), 1, "{:?}", r.diagnostics());
    assert_eq!(all[0].severity, severity);
    assert_eq!(all[0].loc.as_ref().unwrap().line, line);
}

#[test]
fn includes_without_filenames_fail_at_the_directive() {
    for (mode, directive) in
        [("spice", ".inc"), ("spice", ".include"), ("spectre", "include"), ("spectre-spice", ".include")]
    {
        let r = read("0.4", mode, &format!("{directive}\n"));
        diagnosed(
            &r,
            codes::HOST_DIRECTIVE_MISSING_OPERAND,
            Severity::Error,
            if mode == "spectre-spice" { 4 } else { 3 },
        );
        assert!(r.diagnostics().iter().any(|d| d.message.contains(&format!("{directive} needs a filename"))));
    }
}

#[test]
fn a_bare_library_directive_requires_an_operand() {
    for mode in ["spice", "spectre-spice"] {
        let r = read("0.4", mode, ".lib\n");
        diagnosed(&r, codes::HOST_DIRECTIVE_MISSING_OPERAND, Severity::Error, if mode == "spice" { 3 } else { 4 });
        assert!(r.diagnostics().iter().any(|d| d.message.contains("a section name, or a filename and section")));
    }
}

#[test]
fn unmatched_ends_warn_without_dropping_following_instances() {
    for mode in ["spice", "spectre", "spectre-spice"] {
        let body =
            if mode == "spectre" { "ends absent\nR1 (a b) resistor r=1k\n" } else { ".ends absent\nR1 a b 1k\n" };
        let r = read("0.4", mode, body);
        diagnosed(&r, codes::HOST_UNMATCHED_DIRECTIVE, Severity::Warning, if mode == "spectre-spice" { 4 } else { 3 });
        assert!(r.diagnostics().iter().any(|d| d.message.contains("ends absent has no open subcircuit")));
        assert!(r.doc.levels[0].instances.iter().any(|i| i.name == "R1"));
    }
}

#[test]
fn unclosed_protection_warns_at_its_opening_and_stays_unwritable() {
    for (mode, directive) in
        [("spice", ".prot"), ("spice", ".protect"), ("spectre", "protect"), ("spectre-spice", ".prot")]
    {
        let r = read("0.4", mode, &format!("{directive}\n\n"));
        let line = if mode == "spectre-spice" { 4 } else { 3 };
        diagnosed(&r, codes::HOST_UNTERMINATED_REGION, Severity::Warning, line);
        let path = if mode == "spice" { "main.sp" } else { "main.scs" };
        assert!(r.doc.unwritable_at(path, line + 1).is_some());
        assert!(r.diagnostics().iter().any(|d| d.message.contains(&format!("opened by {directive}"))));
    }
}

#[test]
fn unsupported_spice_coupling_warns_and_keeps_the_inductors() {
    for mode in ["spice", "spectre-spice"] {
        let r = read("0.4", mode, "L1 a b 1n\nL2 c d 2n\nK1 L1 L2 0.9\n");
        diagnosed(&r, codes::HOST_ELEMENT_UNSUPPORTED, Severity::Warning, if mode == "spice" { 5 } else { 6 });
        assert!(r.diagnostics().iter().any(|d| d.message.contains("SPICE coupling K1")));
        assert_eq!(r.doc.levels[0].instances.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["L1", "L2"]);
    }
}

#[test]
fn valid_directives_and_closed_regions_add_no_input_diagnostic() {
    for mode in ["spice", "spectre", "spectre-spice"] {
        let body = if mode == "spectre" {
            "protect\nunprotect\nsubckt cell (a b)\nends cell\n"
        } else {
            ".prot\n.unprot\n.subckt cell a b\n.ends cell\n.lib tt\n.endl tt\n"
        };
        let r = read("0.4", mode, body);
        for code in
            [codes::HOST_DIRECTIVE_MISSING_OPERAND, codes::HOST_UNMATCHED_DIRECTIVE, codes::HOST_UNTERMINATED_REGION]
        {
            assert_eq!(count(&r, code), 0, "{mode}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn native_spectre_instances_are_not_treated_as_spice_couplings() {
    let r = read("0.4", "spectre", "K1 (a b) resistor r=1k\n");
    assert_eq!(count(&r, codes::HOST_ELEMENT_UNSUPPORTED), 0);
    assert!(r.doc.levels[0].instances.iter().any(|i| i.name == "K1"));
}

#[test]
fn released_inputs_keep_their_original_silence() {
    for version in ["0.2", "0.3"] {
        for (mode, body) in [
            ("spice", ".include\n.lib\n.ends ghost\n.prot\nK1 L1 L2 0.9\n"),
            ("spectre", "include\nends ghost\nprotect\n"),
            ("spectre-spice", ".include\n.lib\n.ends ghost\n.prot\nK1 L1 L2 0.9\n"),
        ] {
            let r = read(version, mode, body);
            for code in [
                codes::HOST_DIRECTIVE_MISSING_OPERAND,
                codes::HOST_UNMATCHED_DIRECTIVE,
                codes::HOST_UNTERMINATED_REGION,
                codes::HOST_ELEMENT_UNSUPPORTED,
            ] {
                assert_eq!(count(&r, code), 0, "{version}/{mode}: {:?}", r.diagnostics());
            }
        }
    }
}

#[test]
fn includes_use_the_declared_child_version_for_input_issues() {
    for (root, child) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let r = files(
            "main.sp",
            MemoryProvider::new()
                .with("main.sp", &source(root, "spice", ".include child.sp\n"))
                .with("child.sp", &source(child, "spice", ".include\n")),
        );
        assert_eq!(count(&r, codes::HOST_DIRECTIVE_MISSING_OPERAND), usize::from(child == "0.4"));
        if child == "0.4" {
            assert!(
                r.diagnostics()
                    .iter()
                    .any(|d| d.code == codes::HOST_DIRECTIVE_MISSING_OPERAND
                        && d.loc.as_ref().unwrap().file == "child.sp")
            );
        }
    }
}

#[test]
fn library_sections_use_their_own_version_and_skip_other_sections() {
    for child in ["0.3", "0.4"] {
        let root = if child == "0.3" { "0.4" } else { "0.3" };
        let library = format!("* library\n.include\n.lib tt\n* @RNL(type=\"header\") {{ @VERSION(\"{child}\") }}\n.include\n.endl tt\n.lib ff\n.include\n.endl ff\n");
        let r = files(
            "main.sp",
            MemoryProvider::new().with("main.sp", &source(root, "spice", ".lib lib.sp tt\n")).with("lib.sp", &library),
        );
        assert_eq!(
            count(&r, codes::HOST_DIRECTIVE_MISSING_OPERAND),
            usize::from(child == "0.4"),
            "{:?}",
            r.diagnostics()
        );
        if child == "0.4" {
            diagnosed(&r, codes::HOST_DIRECTIVE_MISSING_OPERAND, Severity::Error, 5);
        }
    }
}

#[test]
fn repeated_file_readings_only_diagnose_the_modern_reading_once() {
    let r = files(
        "main.sp",
        MemoryProvider::new()
            .with("main.sp", &source("0.4", "spice", ".include modern.sp\n.include legacy.sp\n"))
            .with("modern.sp", &source("0.4", "spice", ".include shared.sp\n"))
            .with("legacy.sp", &source("0.3", "spice", ".include shared.sp\n"))
            .with("shared.sp", "* shared\n.include\n"),
    );
    diagnosed(&r, codes::HOST_DIRECTIVE_MISSING_OPERAND, Severity::Error, 2);
}

#[test]
fn interpreter_restarts_do_not_duplicate_input_diagnostics() {
    let r = read("0.4", "spice", ".include\n* @RNL { $D = @DEFINE(roles=[]) { %outer }; %outer=(0,0); $D @ATTACH }\n");
    assert_eq!(count(&r, codes::HOST_DIRECTIVE_MISSING_OPERAND), 1);
    assert_eq!(count(&r, codes::DEFINE_BODY_SCENE_REF), 1);
}
