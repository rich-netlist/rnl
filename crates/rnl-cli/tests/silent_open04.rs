//! Visually joined but electrically separate connections have one severity in 0.4.
use rnl_cli::interpret_file;
use rnl_runtime::{diag::Severity, source::MemoryProvider, Interpretation, Options};
use rnl_view::hints::{interpretation_hints, level_hints, Kind};
use std::path::Path;

const BODY: &str = r#"
$S = @DEFINE-SYMBOL { (0,2) @PIN(terminal=1); (0,-2) @PIN(terminal=2); (-1,-1) (1,1) @RECTANGLE };
{#R1 #R2} @SET-SYMBOL $S;
#R1 @PLACE(0,0); #R2 @PLACE(6,0);
&a @WIRE(mode="straight") {#R1:1 (0,4) (3,4)};
&a @WIRE(mode="straight") {(3,4) (6,4) #R2:1};
&b @WIRE(mode="straight") {#R1:2 (0,-4) (6,-4) (6,-2)}
"#;
fn source(version: &str, body: &str) -> String {
    let body = body.lines().map(str::trim).collect::<Vec<_>>().join(" ");
    format!("* connections\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 1k\n* @RNL {{ {body} }}\n")
}
fn read(files: MemoryProvider, problems: bool) -> Interpretation {
    let r = interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, problems, ..Default::default() })
        .unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    r
}

#[test]
fn wire_to_wire_and_wire_to_pin_silent_opens_warn_in_04() {
    let r = read(MemoryProvider::new().with("main.sp", &source("0.4", BODY)), true);
    let hints = interpretation_hints(&r);
    let opens: Vec<_> = hints[0].hints.iter().filter(|h| h.kind == Kind::SilentOpen).collect();
    assert_eq!(opens.len(), 2, "{opens:?}");
    assert!(opens.iter().all(|h| h.severity == Severity::Warning));
    assert!(opens.iter().any(|h| h.message.contains("without naming it")));
    assert!(opens.iter().any(|h| h.message.contains("names no point")));
}

#[test]
fn severity_does_not_require_a_solver_problem() {
    let r = read(MemoryProvider::new().with("main.sp", &source("0.4", BODY)), false);
    assert!(r.problems.is_empty());
    let hints = level_hints(&r.model.levels[0], None);
    let opens: Vec<_> = hints.hints.iter().filter(|h| h.kind == Kind::SilentOpen).collect();
    assert_eq!(opens.len(), 2);
    assert!(opens.iter().all(|h| h.severity == Severity::Warning));
}

#[test]
fn shared_named_vertices_and_explicit_pins_are_connected() {
    let body = format!("%j=(3,4); {}", BODY.replace("(3,4)", "%j").replace("(6,-2)", "#R2:2"));
    let r = read(MemoryProvider::new().with("main.sp", &source("0.4", &body)), true);
    assert!(interpretation_hints(&r)[0].hints.iter().all(|h| h.kind != Kind::SilentOpen));
}

#[test]
fn an_included_subcircuit_warns_too() {
    let main = "* main\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include child.sp\n";
    let child = source("0.4", BODY).replace("R1 a b 1k", ".subckt child a b\nR1 a b 1k") + ".ends child\n";
    let r = read(MemoryProvider::new().with("main.sp", main).with("child.sp", &child), true);
    let hints = interpretation_hints(&r);
    let opens: Vec<_> = hints.iter().flat_map(|h| &h.hints).filter(|h| h.kind == Kind::SilentOpen).collect();
    assert_eq!(opens.len(), 2, "{opens:?}");
    assert!(opens.iter().all(|h| h.severity == Severity::Warning));
}
