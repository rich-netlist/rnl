//! The members of ASSERT-TOPOLOGY (15.4): a pin in a black box brings the terminals it binds, and a
//! failed object among the members, written in a set too, nested or named, fails the assertion as a
//! whole, as for the assertions of 15.3 (7.3).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// R5 on x–y, R6 on y–q and R7 on q–z, R5 drawn with a symbol whose pins are p and n; the last
/// record writes `body`.
fn read(body: &str) -> Interpretation {
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR5 x y 1k\nR6 y q 1k\nR7 q z 1k\n\
         * @RNL {{ $RS = @DEFINE-SYMBOL {{ (0, 1) @PIN(terminal=1, direction=\"up\") => %p; \
         (0, -1) @PIN(terminal=2, direction=\"down\") => %n; (0, 0) @CIRCLE(radius=1) }}; #R5 @SET-SYMBOL $RS; {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

fn of(r: &Interpretation, severity: Severity) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| (d.code, r.sink.render(d))).collect()
}

#[test]
fn a_pin_in_a_black_box_brings_the_terminals_it_binds() {
    // the box of R5's pins is on y, which R6 shares: in series
    let r = read("{{#R5::p #R5::n} #R6} @ASSERT-TOPOLOGY(\"series\")");
    assert_eq!(of(&r, Severity::Error), Vec::new());
    assert_eq!(of(&r, Severity::Violation), Vec::new());
    // R7 shares no net with it
    let r = read("{{#R5::p #R5::n} #R7} @ASSERT-TOPOLOGY(\"series\")");
    let v = of(&r, Severity::Violation);
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].0, codes::VIOLATION_TOPOLOGY);
}

#[test]
fn a_failed_member_fails_the_assertion() {
    for body in [
        "{#R9 #R6 #R7} @ASSERT-TOPOLOGY(\"series\")",
        "{{#R9} #R6} @ASSERT-TOPOLOGY(\"series\")",
        "%s = {#R9 #R6}; %s @ASSERT-TOPOLOGY(\"series\")",
    ] {
        let r = read(body);
        let e: Vec<Code> = of(&r, Severity::Error).into_iter().map(|(c, _)| c).collect();
        assert!(e.contains(&codes::ASSERT_MEMBER_FAILED), "{body}: {e:?}");
        // it checks nothing
        assert_eq!(of(&r, Severity::Violation), Vec::new(), "{body}");
    }
}
