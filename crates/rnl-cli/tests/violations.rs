//! Violations (7.6): a statement about the circuit that does not hold, reported as an error is and
//! ranked after errors, while nothing fails. The first kind: the CURRENT statements of a net that
//! send all its current one way, in a file of 0.4 (15.2).

use std::path::Path;
use std::process::Command;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version` whose first record writes `statements`; the subcircuit `cell` has a
/// record of its own, `inside`.
fn netlist(version: &str, statements: &str, inside: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nR1 a b 1k\nR2 b 0 1k\nX1 a cell\n\
         .subckt cell p\nR3 p 0 1k\n* @RNL {{ {inside} }}\n.ends cell\n* @RNL {{ {statements} }}\n.end\n"
    )
}

fn read(text: &str) -> Interpretation {
    let options = Options { std: false, ..Options::default() };
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), &options).unwrap()
}

/// The violations, as `rnl check` prints them.
fn violations(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Violation).map(|d| r.sink.render(d)).collect()
}

#[test]
fn a_net_whose_current_goes_one_way_is_a_violation() {
    // both terminals on b take current out of it: it has nowhere to go
    let r = read(&netlist("0.4", "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"in\")", ""));
    let v = violations(&r);
    assert_eq!(v.len(), 1, "{v:?}");
    // at the later CURRENT, with the kind, the net and its terminals
    assert!(v[0].starts_with("violation t.sp:10:"), "{v:?}");
    assert!(v[0].contains(": RNL6003 violation-current-one-way: the current of &b goes one way"), "{v:?}");
    assert!(v[0].contains("#R1:n, #R2:p") && v[0].contains("out of the net"), "{v:?}");
    let d = r.diagnostics().into_iter().find(|d| d.severity == Severity::Violation).unwrap();
    assert_eq!(d.code, codes::VIOLATION_CURRENT_ONE_WAY);
    // nothing fails: no error, and the CURRENTs stay in effect
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].currents.len(), 2);
}

#[test]
fn a_net_whose_current_can_go_somewhere_is_none() {
    for statements in [
        // one in, one out
        "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"out\")",
        // a terminal on the net without CURRENT, or unspecified, leaves it unchecked
        "#R1:2 @CURRENT(\"in\")",
        "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"unspecified\")",
        // the later CURRENT of a terminal is the one in effect
        "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"in\"); #R2:1 @CURRENT(\"out\")",
    ] {
        assert_eq!(violations(&read(&netlist("0.4", statements, ""))), Vec::<String>::new(), "{statements}");
    }
}

#[test]
fn a_port_with_in_brings_current_into_the_net() {
    // the port p brings current in and R3 takes it out: it goes somewhere
    let r = read(&netlist("0.4", "", "#:p @CURRENT(\"in\"); #R3:1 @CURRENT(\"in\")"));
    assert_eq!(violations(&r), Vec::<String>::new());
    // both bring it in
    let r = read(&netlist("0.4", "", "#:p @CURRENT(\"in\"); #R3:1 @CURRENT(\"out\")"));
    let v = violations(&r);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].contains("the current of &p goes one way") && v[0].contains("into the net"), "{v:?}");
}

#[test]
fn a_global_net_is_not_checked() {
    // R2's other terminal is on 0, and R3's on the net g that .global declares: the current may
    // leave through the terminals of other levels
    for (decl, net) in [("", "0"), (".global g\n", "g")] {
        let text = netlist("0.4", "#R2:2 @CURRENT(\"in\")", "").replace("R2 b 0 1k", &format!("{decl}R2 b {net} 1k"));
        assert_eq!(violations(&read(&text)), Vec::<String>::new(), "{net}");
    }
}

#[test]
fn files_of_0_2_and_0_3_have_no_such_check() {
    for version in ["0.2", "0.3"] {
        let r = read(&netlist(version, "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"in\")", ""));
        assert_eq!(violations(&r), Vec::<String>::new(), "{version}");
    }
}

#[test]
fn check_counts_violations_and_fails_on_them() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-violations");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.sp");
    std::fs::write(&file, netlist("0.4", "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"in\")", "")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(["check", "--no-std"]).arg(&file).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("violation ") && stdout.contains("RNL6003 violation-current-one-way"), "{stdout}");
    assert!(stdout.ends_with("0 errors, 1 violations, 0 warnings, 2 hints\n"), "{stdout}");
}

#[test]
fn the_document_gives_the_violation() {
    let text = netlist("0.4", "#R1:2 @CURRENT(\"in\"); #R2:1 @CURRENT(\"in\")", "");
    let options = rnl_view::BuildOptions {
        runtime: Options { std: false, ..Options::default() },
        ..rnl_view::BuildOptions::default()
    };
    let d = rnl_view::build("t.sp", &MemoryProvider::new().with("t.sp", &text), &options).unwrap();
    let v: Vec<_> = d.diagnostics.iter().filter(|d| d.severity == "violation").collect();
    assert_eq!(v.len(), 1, "{:?}", d.diagnostics);
    assert_eq!((v[0].code.as_deref(), v[0].name), (Some("RNL6003"), "violation-current-one-way"));
}
