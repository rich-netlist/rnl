//! Terminals written with digits (6.4): in a file of 0.4 a terminal made only of digits is a number,
//! also when it is quoted, wherever a terminal is named, and a number reaches a terminal by its place
//! only, so in `.subckt amp 3 1 2` `#X1:1` is the port named 3. A file of 0.3 reads them as it did.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version`: X1 of the subcircuit `amp`, whose ports are named 3, 1 and 2, on a, b
/// and c, and M1 on d, g, s, b; its last record writes `body`.
fn read(version: &str, body: &str) -> Interpretation {
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.subckt amp 3 1 2\nR1 3 1 1k\nR2 1 2 1k\n.ends\n\
         X1 a b c amp\n.model nch nmos level=1\nM1 d g s b nch\n* @RNL {{ {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

fn of(r: &Interpretation, severity: Severity) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| (d.code, r.sink.render(d))).collect()
}

/// Whether `body` in a file of 0.4 holds: no error and no violation.
fn holds(body: &str) -> bool {
    let r = read("0.4", body);
    assert_eq!(of(&r, Severity::Error), Vec::new(), "{body}");
    of(&r, Severity::Violation).is_empty()
}

#[test]
fn a_number_reaches_a_terminal_by_its_place() {
    for (reference, net) in [("#X1:1", "a"), ("#X1:\"1\"", "a"), ("#X1:3", "c"), ("#X1:\"3\"", "c"), ("#M1:\"2\"", "g")]
    {
        assert!(holds(&format!("{reference} @ASSERT-TERMINAL(net=\"{net}\")")), "{reference} is on {net}");
        assert!(!holds(&format!("{reference} @ASSERT-TERMINAL(net=\"b\")")), "{reference} is not on b");
    }
}

#[test]
fn a_string_of_digits_in_the_terminal_condition_is_a_number() {
    assert!(holds("#X1 @SELECT-TERMINAL(terminal=\"3\") => @ASSERT-TERMINAL(net=\"c\")"));
    assert!(holds("#X1 @SELECT-TERMINAL(terminal=3) => @ASSERT-TERMINAL(net=\"c\")"));
    assert!(holds("#X1:3 @ASSERT-TERMINAL(terminal=\"3\")"));
    assert!(!holds("#X1:1 @ASSERT-TERMINAL(terminal=\"3\")"));
}

#[test]
fn a_file_of_0_3_reads_them_as_it_did() {
    // a quoted terminal is a name there, and M1 has no terminal named 1
    let e = of(&read("0.3", "#M1:\"1\" @CURRENT(\"in\")"), Severity::Error);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].0, codes::REF_NO_TERMINAL);
    assert_eq!(of(&read("0.4", "#M1:\"1\" @CURRENT(\"in\")"), Severity::Error), Vec::new());
}
