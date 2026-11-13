//! BUNDLE and WIRE take objects written out (10.4, 11.8, 13.1): in a file of 0.4, given the result
//! of a selection, or a set that depends on one, they fail. A file of 0.3 reads them as it did.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version` with R1 to R3 between a and b, the halves of the bus instance XI0 and the
/// subcircuit `cell`, whose last record writes `body`.
fn read(version: &str, body: &str) -> Interpretation {
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 1k\nR3 a b 1k\n\
         XI0<1> a b cell\nXI0<0> a b cell\n.subckt cell p q\nR1 p q 1k\n.ends\n* @RNL {{ {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

fn errors(r: &Interpretation) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| (d.code, r.sink.render(d))).collect()
}

fn codes_of(r: &Interpretation) -> Vec<Code> {
    errors(r).into_iter().map(|(c, _)| c).collect()
}

#[test]
fn bundle_given_a_selection_fails() {
    let r = read("0.4", "# @SELECT-INSTANCE(name=\"XI0*\") => @BUNDLE");
    let e = errors(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].0, codes::KEYWORD_BUNDLE_SELECTION);
    assert!(e[0].1.contains("BUNDLE takes objects written out, not the result of a selection"), "{e:?}");
    assert!(r.model.levels[0].items.iter().all(|i| i.kind != "bundle"));
    // a set that a selection extends, and a set bound to a selection
    for body in [
        "%t = {#XI0<1>}; # @SELECT-INSTANCE(name=\"XI0<0>\") => @ADD-TO %t; %t @BUNDLE",
        "%t = # @SELECT-INSTANCE(name=\"XI0*\"); %t @BUNDLE",
    ] {
        let e = errors(&read("0.4", body));
        assert!(e.iter().any(|(c, _)| *c == codes::KEYWORD_BUNDLE_SELECTION), "{body}: {e:?}");
    }
}

#[test]
fn wire_given_a_selection_fails() {
    for (body, said) in [
        ("%s = # @SELECT-PIN(net=\"a\"); &a @WIRE %s", "the result of a selection"),
        ("%s = # @SELECT-TERMINAL(net=\"a\"); &a @WIRE %s", "the result of a selection"),
        ("%u = {(0, 0) (1, 1)}; # @SELECT-PIN(net=\"a\") => @ADD-TO %u; &a @WIRE %u", "a set that depends on one"),
    ] {
        let e = errors(&read("0.4", body));
        assert_eq!(e.iter().filter(|(c, _)| *c == codes::WIRE_SELECTION).count(), 1, "{body}: {e:?}");
        assert!(e.iter().any(|(_, l)| l.contains(&format!("WIRE does not take {said}"))), "{body}: {e:?}");
    }
}

#[test]
fn objects_written_out_are_taken() {
    let r = read("0.4", "%b = #XI0<1:0> @BUNDLE; %b @PLACE(0, 0); #R1 @PLACE(0, 5); &a @WIRE {(0, 0) (0, 5)}");
    assert_eq!(codes_of(&r), Vec::<Code>::new());
    assert!(r.model.levels[0].items.iter().any(|i| i.kind == "bundle"));
}

#[test]
fn a_file_of_0_3_reads_them_as_it_did() {
    // a bundle made from a selection, with no error of this kind
    let r = read("0.3", "# @SELECT-INSTANCE(name=\"XI0*\") => @BUNDLE => %b; %b @PLACE(0, 0)");
    assert!(!codes_of(&r).contains(&codes::KEYWORD_BUNDLE_SELECTION), "{:?}", errors(&r));
    assert!(r.model.levels[0].items.iter().any(|i| i.kind == "bundle"));
    let r = read("0.3", "%s = # @SELECT-TERMINAL(net=\"a\"); &a @WIRE %s");
    assert!(!codes_of(&r).contains(&codes::WIRE_SELECTION), "{:?}", errors(&r));
}
